// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use std::time::Instant;

use arbutil::format;
use eyre::{bail, Result};
use humantime::Duration;
use prover::{binary::WasmBinary, programs::prelude::CompileConfig};
use stylus::native::{self, NativeInstance};

mod util;
mod wasm;

fn main() -> Result<()> {
    /*affinity::set_thread_affinity([2]).unwrap();
    let core = affinity::get_thread_affinity().unwrap();
    println!("Affinity {}: {core:?}", std::process::id());*/

    let kilos = 1024;
    let mut worst = 0.;

    for _ in 0.. {
        let entropy = util::random_vec(rand::random::<usize>() % (128 * kilos));
        let wasm = wasm::random(&entropy)?;
        let size = wasm.len() / kilos;

        if size < 5 {
            continue;
        }

        let gas = match sample(&wasm) {
            Ok(gas) => gas,
            Err(error) => {
                //println!("{error}");
                continue;
            }
        };

        let max_cost = gas * 128. * kilos as f64;
        if worst < max_cost {
            println!("Gas {size}kb {:.2} {:.2}m", gas, max_cost / 1000_000.);
        }
        if worst < max_cost {
            worst = max_cost;
        }
        if worst > 5_000_000. {
            println!("{}", wat(&wasm)?);
            println!("{size}");
            for _ in 0..4 {
                println!("rerun {}", sample(&wasm)?);
            }
            bail!("too much time!");
        }
    }
    Ok(())
}

struct Run {
    size: usize,
    tables: usize,
    funcs: usize,
    globals: usize,
    exports: usize,
    footprint: u16,
    bin_time: Duration,
    mod_time: Duration,
}

fn sample(wasm: &[u8]) -> Result<Run> {
    let before = Instant::now();
    let compile = CompileConfig::version(1, false);
    let (bin, _, footprint) = WasmBinary::parse_user(&wasm, 128, &compile)?;
    let bin_time = before.elapsed();
    native::module(wasm, compile)?;
    let mod_time = before.elapsed();

    Ok(Run {
        size: wasm.len(),
        tables: bin.tables.len(),
        funcs: bin.functions.len(),
        globals: bin.globals.len(),
        exports: bin.exports.len(),
        footprint,
        bin_time: bin_time.into(),
        mod_time: mod_time.into(),
    })
}

fn wat(wasm: &[u8]) -> Result<String> {
    let text = wasmprinter::print_bytes(wasm);
    text.map_err(|x| eyre::eyre!(x))
}
