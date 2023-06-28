// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use crate::{evm_api::PanicApi, util, wasm};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

use eyre::Result;
use prover::programs::{prelude::*, start::StartlessMachine};
use stylus::native::NativeInstance;

pub fn record(path: &Path, period: Duration) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(path)?;

    affinity::set_thread_affinity([1]).unwrap();
    let core = affinity::get_thread_affinity().unwrap();
    println!("Affinity {}: {core:?}", std::process::id());

    let start = Instant::now();
    let mut valids = 0;
    let mut errors = 0;

    loop {
        let update = trial(&mut file)?;
        match update {
            true => valids += 1,
            false => errors += 1,
        }
        //return Ok(());

        if start.elapsed() >= period {
            println!("Counts: {} {}", valids, valids + errors);
            return Ok(());
        }
    }
}

fn trial(file: &mut File) -> Result<bool> {
    let entropy = util::random_vec(rand::random::<usize>() % 512);
    let wasm_bytes = wasm::random(&entropy)?;
    //println!("WAT: {}", wat(&wasm_bytes)?);

    if let Err(_error) = wasm::validate(&wasm_bytes) {
        println!("Error: {}", _error);
        return Ok(false);
    }

    let mut compile = CompileConfig::version(1, true);
    compile.pricing.costs = |_| 1;
    compile.pricing.memory_copy_ink = 1;
    compile.pricing.memory_fill_ink = 1;
    compile.debug.count_ops = true;

    let config = StylusConfig::default();

    //let timer = Instant::now();
    /*let mut native =
    unsafe { NativeInstance::deserialize(&module, compile, PanicApi, EvmData::default())? };*/
    let mut native = NativeInstance::from_wasm_wat(&wasm_bytes, PanicApi, &compile, config)?;
    native.set_ink(5_000_000);
    native.set_stack(1024);

    let start = native.get_start()?;

    let (elapsed, success) = {
        let timer = Instant::now();
        let outcome = start.call(&mut native.store);
        let elapsed = timer.elapsed().as_nanos();
        let success = outcome.is_ok();
        (elapsed, success)
    };

    /*if success {
        println!("WAT: {}", wat(&wasm_bytes)?);
    }*/

    write!(file, "{elapsed} {success}")?;
    for (op, count) in native.operator_counts()? {
        write!(file, " {} {}", op.0, count)?;
    }
    writeln!(file)?;

    Ok(success)
}

fn _wat(wasm: &[u8]) -> Result<String> {
    let text = wasmprinter::print_bytes(wasm);
    text.map_err(|x| eyre::eyre!(x))
}
