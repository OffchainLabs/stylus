// Copyright 2021-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![no_main]
#![allow(clippy::field_reassign_with_default, unused_imports)]


mod util;
mod wasm;

use libfuzzer_sys::{Corpus, fuzz_target};
use prover::{binary::parse, programs::{start::StartlessMachine, config::{StylusConfig, StylusDebugConfig}, counter::CountingMachine}};
use std::path::Path;
use stylus::{env::WasmEnv, stylus::{instance_from_module, NativeInstance}};
use util::{wat, warn, fail};
use wasmer::Module;
use wasmparser::Operator;

fuzz_target!(|data: &[u8]| -> Corpus {
    let wasm_data  = wasm::random(data, 0);
    if !wasm::validate(&wasm_data) {
        return Corpus::Keep;
    }

    let enable_counter = false;
    let gas_limit = 200;

    let mut config = StylusConfig::default();
    if enable_counter {
        config.add_debug_params();
    }
    config.costs = |_: &Operator| -> u64 {1};
    config.start_gas = gas_limit;
    config.pricing.wasm_gas_price = 1;

    let module = match Module::new(&config.store(), wasm_data.clone()) {
        Ok(module) => module,
        Err(_) => {
            return Corpus::Keep;
        }
    };

    let env = WasmEnv::new(config.clone(), vec![]);
    let mut instance = match instance_from_module(module, config.store(), env) {
        Ok(instance) => instance,
        Err(err) => {
            let err = err.to_string();
            if err.contains("Missing export memory") ||
               err.contains("out of bounds memory access") ||
               err.contains("Incompatible Export Type") ||
               err.contains("WebAssembly transaction error") ||
               err.contains("out of bounds table access") {
                return Corpus::Keep;
            }
            println!("{}", wat!(&wasm_data));
            panic!("Failed to create instance: {err}");
        }
    };

    let starter = match instance.get_start() {
        Ok(starter) => starter,
        Err(err) => {
            println!("{}", wat!(&wasm_data));
            panic!("Failed to get start: {err}");
        }
    };
    if let Err(e) = starter.call(&mut instance.store) {
        println!("{}", wat!(&wasm_data));
        panic!("Failed to get start: {e}");
    }
    println!("Finished main");

    if enable_counter {
        let counts = match instance.operator_counts() {
            Ok(counts) => counts,
            Err(err) => {
                println!("{}", wat!(&wasm_data));
                panic!("Failed to get operator counts: {err}");
            }
        };
        for (op, count) in counts.into_iter() {
            println!("{op}\t{count}\n");
        }
    }

    Corpus::Keep
});
