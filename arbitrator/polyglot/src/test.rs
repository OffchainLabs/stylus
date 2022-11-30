// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![cfg(test)]

use crate::{
    machine::{self, WasmEnvArc},
    ExecOutcome, ExecPolyglot,
};

use brotli2::read::{BrotliDecoder, BrotliEncoder};
use eyre::{bail, Result};
use prover::{
    machine::MachineStatus,
    programs::{
        depth::DepthCheckedMachine,
        meter::{MachineMeter, MeteredMachine},
        GlobalMod, PolyglotConfig,
    },
    Machine, Value,
};
use sha3::{Digest, Keccak256};
use std::{
    io::Read,
    time::{Duration, Instant},
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
    let mut config = PolyglotConfig::default();
    config.costs = expensive_add;
    config.max_depth = 1024;

    let (module, store) = machine::instrument(&wasm, &config)?;
    let mut instance = machine::create(&module, &store, WasmEnvArc::default())?;
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

    let mut machine = Machine::from_polyglot_binary(&wasm, false, &config)?;
    assert_eq!(machine.get_status(), MachineStatus::Running);
    assert_eq!(machine.gas_left(), MachineMeter::Ready(0));

    let args = vec![Value::I32(32)];
    let status = machine
        .call_function("user", "add_one", &args)?
        .unwrap_err();
    assert_eq!(status, MachineStatus::Errored);
    assert_eq!(machine.gas_left(), MachineMeter::Exhausted);

    machine.set_gas(1000);
    assert_eq!(machine.gas_left(), MachineMeter::Ready(1000));
    let output = machine.call_function("user", "add_one", &args)?.unwrap();
    assert_eq!(output, vec![Value::I32(33)]);
    assert_eq!(machine.gas_left(), MachineMeter::Ready(900));
    Ok(())
}

#[test]
fn test_depth() -> Result<()> {
    let wasm = std::fs::read("../jit/programs/pure/main.wat")?;
    let mut config = PolyglotConfig::default();
    config.max_depth = 32;

    let (module, store) = machine::instrument(&wasm, &config)?;
    let mut instance = machine::create(&module, &store, WasmEnvArc::default())?;
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

    // show that a successful call reclaims the stack
    instance.reset_stack();
    let add_one = instance.exports.get_function("add_one")?;
    let add_one = add_one.native::<i32, i32>().unwrap();
    assert_eq!(add_one.call(32)?, 33);
    assert_eq!(instance.stack_space_left(), 64);
    Ok(())
}

#[test]
fn test_depth_arbitrator() -> Result<()> {
    let wasm = std::fs::read("../jit/programs/pure/main.wat")?;
    let wasm = wasmer::wat2wasm(&wasm)?;
    let mut config = PolyglotConfig::default();
    config.start_gas = 1024;
    config.max_depth = 32;

    let mut machine = Machine::from_polyglot_binary(&wasm, false, &config)?;
    let status = machine
        .call_function("user", "recurse", &vec![])?
        .unwrap_err();
    assert_eq!(status, MachineStatus::Errored);
    assert_eq!(machine.get_global("depth")?, Value::I32(5)); // 32 capacity / 6-word frame => 5 calls

    machine.set_stack_limit(48);
    assert_eq!(machine.stack_space_left(), 16);
    assert_eq!(machine.stack_size(), 32);

    machine.reset_stack();
    machine.set_stack_limit(64);
    assert_eq!(machine.stack_space_left(), 64);

    let status = machine
        .call_function("user", "recurse", &vec![])?
        .unwrap_err();
    assert_eq!(status, MachineStatus::Errored);
    let program_depth = machine.get_global("depth")?;
    assert_eq!(program_depth, Value::I32(5 + 10)); // 64 more capacity / 6-word frame => 10 more calls
    Ok(())
}

#[test]
pub fn test_sha3() -> Result<()> {
    let wasm = std::fs::read("programs/sha3/target/wasm32-unknown-unknown/release/sha3.wasm")?;
    let mut config = PolyglotConfig::default();
    config.costs = |_: &Operator| 1;
    config.start_gas = 1_000_000;

    let time = Instant::now();
    let preimage = "°º¤ø,¸¸,ø¤º°`°º¤ø,¸,ø¤°º¤ø,¸¸,ø¤º°`°º¤ø,¸ nyan nyan ~=[,,_,,]:3 nyan nyan";
    let mut hasher = Keccak256::new();
    hasher.update(preimage);
    let hash = hasher.finalize().to_vec();
    println!("native:    {}", format_time(time.elapsed()));

    let time = Instant::now();
    let env = WasmEnvArc::new(preimage.as_bytes(), 1000);
    let (module, store) = machine::instrument(&wasm, &config)?;
    let compressed_wasm = compress(&wasm, 0);
    let compressed_module = compress(&module, 0);
    println!("Ploy load: {}", format_time(time.elapsed()));

    let time = Instant::now();
    let decompressed_module = decompress(&compressed_module);
    assert_eq!(module, decompressed_module);
    let mut instance = machine::create(&module, &store, env.clone())?;
    match instance.run_main(env.clone())? {
        ExecOutcome::Success(output) => assert_eq!(output, hash),
        failure => bail!("call failed: {}", failure),
    }
    println!("Poly main: {}", format_time(time.elapsed()));

    let time = Instant::now();
    let mut machine = Machine::from_polyglot_binary(&wasm, false, &config)?;
    println!("Mach load: {}", format_time(time.elapsed()));

    let time = Instant::now();
    match machine.run_main(env)? {
        ExecOutcome::Success(output) => assert_eq!(hex::encode(output), hex::encode(hash)),
        failure => bail!("call failed: {}", failure),
    }
    println!("Mach main: {}", format_time(time.elapsed()));

    println!("Size {}KB {}KB", wasm.len() / 1024, module.len() / 1024);
    println!(
        "Size {}KB {}KB",
        compressed_wasm.len() / 1024,
        compressed_module.len() / 1024
    );

    assert_eq!(instance.gas_left(), machine.gas_left());
    Ok(())
}

#[test]
pub fn test_eddsa() -> Result<()> {
    use ed25519_dalek::{Keypair, Signer, Verifier};
    use rand::rngs::OsRng;

    let wasm = std::fs::read("programs/monocypher/eddsa.wasm")?;
    let mut config = PolyglotConfig::default();
    config.costs = |_: &Operator| 1;
    config.start_gas = 10_000_000;

    let mut rng = OsRng {};
    let message = "✲´*。.❄¨¯`* ✲。(╯^□^)╯ <(yay, it's snowing!) ✲。❄。*。¨¯`*✲".as_bytes();
    let keypair: Keypair = Keypair::generate(&mut rng);
    let signature = keypair.sign(message);

    let mut args = signature.to_bytes().to_vec();
    args.extend(keypair.public.to_bytes());
    args.extend(message);
    let env = WasmEnvArc::new(&args, 1000);

    let time = Instant::now();
    assert!(keypair.public.verify(message, &signature).is_ok());
    println!("Native:    {}", format_time(time.elapsed()));

    let time = Instant::now();
    let (module, store) = machine::instrument(&wasm, &config)?;
    println!("Ploy load: {}", format_time(time.elapsed()));

    let time = Instant::now();
    let mut instance = machine::create(&module, &store, env.clone())?;
    match instance.run_main(env.clone())? {
        ExecOutcome::Success(output) => assert_eq!(output, vec![]),
        ExecOutcome::Revert(output) => {
            bail!("reverted with {}", hex::encode(output))
        }
        failure => bail!("call failed: {}", failure),
    }
    println!("Poly main: {}", format_time(time.elapsed()));
    Ok(())
}

fn compress(data: &[u8], level: u32) -> Vec<u8> {
    let mut output = vec![];
    let mut compressor = BrotliEncoder::new(data, level);
    compressor.read_to_end(&mut output).unwrap();
    output
}

fn decompress(data: &[u8]) -> Vec<u8> {
    let mut output = vec![];
    let mut decompressor = BrotliDecoder::new(data);
    decompressor.read_to_end(&mut output).unwrap();
    output
}

fn format_time(span: Duration) -> String {
    use arbutil::color;
    let mut span = span.as_nanos() as f64;
    let mut unit = 0;
    let units = vec!["ns", "μs", "ms", "s"];
    let scale = vec![1000., 1000., 1000., 1000.];
    let colors = vec![color::MINT, color::MINT, color::YELLOW, color::RED];
    while span > 100. {
        span /= scale[unit];
        unit += 1;
    }
    color::color(
        colors[unit],
        format!("{:6}", format!("{:.1}{}", span, units[unit])),
    )
}
