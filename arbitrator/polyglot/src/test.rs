// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![cfg(test)]

use crate::{
    machine,
    middlewares::{
        self,
        depth,
        meter::{self, set_gas, MachineMeter},
    },
};

use eyre::Result;
use wasmparser::Operator;

#[test]
fn test_gas() -> Result<()> {
    let costs = |op: &Operator| -> u64 {
        match op {
            Operator::I32Add => 100,
            _ => 0,
        }
    };

    let wasm = std::fs::read("../jit/programs/pure/main.wat")?;
    let instance = machine::create(&wasm, costs, 0, 1024)?;
    let add_one = instance.exports.get_function("add_one")?;
    let add_one = add_one.native::<i32, i32>().unwrap();

    assert_eq!(meter::gas_left(&instance), MachineMeter::Ready(0));
    assert!(add_one.call(32).is_err());
    assert_eq!(meter::gas_left(&instance), MachineMeter::Exhausted);

    set_gas(&instance, 1000);
    assert_eq!(meter::gas_left(&instance), MachineMeter::Ready(1000));
    assert_eq!(add_one.call(32)?, 33);
    assert_eq!(meter::gas_left(&instance), MachineMeter::Ready(900));
    Ok(())
}

#[test]
fn test_depth() -> Result<()> {
    let wasm = std::fs::read("../jit/programs/pure/main.wat")?;
    let costs = |_: &Operator| 0;
    let instance = machine::create(&wasm, costs, 1024, 32)?;
    let recurse = instance.exports.get_function("recurse")?;
    let recurse = recurse.native::<(), ()>().unwrap();

    assert!(recurse.call().is_err());
    assert_eq!(depth::stack_space_remaining(&instance), 0);
    assert_eq!(depth::stack_size(&instance), 32);

    let program_depth: u32 = middlewares::get_global(&instance, "depth");
    assert_eq!(program_depth, 5); // 32 capacity / 6-word frame => 5 calls

    depth::set_stack_limit(&instance, 48);
    assert_eq!(depth::stack_space_remaining(&instance), 16);
    assert_eq!(depth::stack_size(&instance), 32);

    depth::reset_stack(&instance);
    depth::set_stack_limit(&instance, 64);
    assert_eq!(depth::stack_space_remaining(&instance), 64);

    assert!(recurse.call().is_err());
    assert_eq!(depth::stack_space_remaining(&instance), 0);
    let program_depth: u32 = middlewares::get_global(&instance, "depth");
    assert_eq!(program_depth, 5 + 10); // 64 more capacity / 6-word frame => 10 more calls
    Ok(())
}
