// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![cfg(test)]

use crate::machine;

use eyre::Result;
use prover::{
    machine::MachineStatus,
    middlewares::{
        depth::DepthCheckedMachine,
        meter::{MachineMeter, MeteredMachine},
        GlobalMod, PolyglotConfig,
    },
    Machine, Value,
};
use wasmparser::Operator;

fn expensive_add(op: &Operator) -> u64 {
    match op {
        Operator::I32Add => 100,
        _ => 0,
    }
}

#[test]
fn test_gas() -> Result<()> {
    let wasm = std::fs::read("../jit/programs/pure/main.wat")?;
    let mut instance = machine::create(&wasm, expensive_add, 0, 1024)?;
    let add_one = instance.exports.get_function("add_one")?;
    let add_one = add_one.native::<i32, i32>().unwrap();

    assert_eq!(instance.gas_left(), MachineMeter::Ready(0));
    assert!(add_one.call(32).is_err());
    assert_eq!(instance.gas_left(), MachineMeter::Exhausted);

    instance.set_gas(1000);
    assert_eq!(instance.gas_left(), MachineMeter::Ready(1000));
    assert_eq!(add_one.call(32)?, 33);
    assert_eq!(instance.gas_left(), MachineMeter::Ready(900));
    Ok(())
}

#[test]
fn test_gas_arbitrator() -> Result<()> {
    let wasm = std::fs::read("../jit/programs/pure/main.wat")?;
    let wasm = wasmer::wat2wasm(&wasm)?;
    let mut config = PolyglotConfig::default();
    config.costs = expensive_add;

    let mut machine = Machine::from_polyglot_binary(&wasm, &config)?;
    assert_eq!(machine.get_status(), MachineStatus::Running);
    assert_eq!(machine.gas_left(), MachineMeter::Ready(0));

    let args = vec![Value::I32(32)];
    let status = machine.call_function("add_one", &args)?.unwrap_err();
    assert_eq!(status, MachineStatus::Errored);
    assert_eq!(machine.gas_left(), MachineMeter::Exhausted);

    machine.set_gas(1000);
    assert_eq!(machine.gas_left(), MachineMeter::Ready(1000));
    let output = machine.call_function("add_one", &args)?.unwrap();
    assert_eq!(output, vec![Value::I32(33)]);
    assert_eq!(machine.gas_left(), MachineMeter::Ready(900));
    Ok(())
}

#[test]
fn test_depth() -> Result<()> {
    let wasm = std::fs::read("../jit/programs/pure/main.wat")?;
    let costs = |_: &Operator| 0;
    let mut instance = machine::create(&wasm, costs, 1024, 32)?;
    let recurse = instance.exports.get_function("recurse")?;
    let recurse = recurse.native::<(), ()>().unwrap();

    assert!(recurse.call().is_err());
    assert_eq!(instance.stack_space_left(), 0);
    assert_eq!(instance.stack_size(), 32);

    let program_depth: u32 = instance.get_global("depth");
    assert_eq!(program_depth, 5); // 32 capacity / 6-word frame => 5 calls

    instance.set_stack_limit(48);
    assert_eq!(instance.stack_space_left(), 16);
    assert_eq!(instance.stack_size(), 32);

    instance.reset_stack();
    instance.set_stack_limit(64);
    assert_eq!(instance.stack_space_left(), 64);

    assert!(recurse.call().is_err());
    assert_eq!(instance.stack_space_left(), 0);
    let program_depth: u32 = instance.get_global("depth");
    assert_eq!(program_depth, 5 + 10); // 64 more capacity / 6-word frame => 10 more calls
    Ok(())
}

#[test]
fn test_depth_arbitrator() -> Result<()> {
    let wasm = std::fs::read("../jit/programs/pure/main.wat")?;
    let wasm = wasmer::wat2wasm(&wasm)?;
    let mut config = PolyglotConfig::default();
    config.start_gas = 1024;
    config.max_depth = 32;

    let mut machine = Machine::from_polyglot_binary(&wasm, &config)?;
    let status = machine.call_function("recurse", &vec![])?.unwrap_err();
    assert_eq!(status, MachineStatus::Errored);
    assert_eq!(machine.get_global("depth")?, Value::I32(5)); // 32 capacity / 6-word frame => 5 calls

    machine.set_stack_limit(48);
    assert_eq!(machine.stack_space_left(), 16);
    assert_eq!(machine.stack_size(), 32);

    machine.reset_stack();
    machine.set_stack_limit(64);
    assert_eq!(machine.stack_space_left(), 64);

    let status = machine.call_function("recurse", &vec![])?.unwrap_err();
    assert_eq!(status, MachineStatus::Errored);
    let program_depth = machine.get_global("depth")?;
    assert_eq!(program_depth, Value::I32(5 + 10)); // 64 more capacity / 6-word frame => 10 more calls
    Ok(())
}
