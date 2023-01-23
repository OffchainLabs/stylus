// Copyright 2021-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![no_main]
#![allow(clippy::field_reassign_with_default)]

use libfuzzer_sys::fuzz_target;

mod util;
mod wasm;

use util::{warn};
use prover::programs::{config::{StylusConfig, StylusDebugConfig}, counter::CountingMachine};
use stylus::stylus::instance_from_module;
use stylus::env::WasmEnv;
use wasmer::Module;

fuzz_target!(|data: &[u8]| {
    let mut config = StylusConfig::default();
    config.debug = Some(StylusDebugConfig::default());

    let wasm_data  = wasm::random(data, 0);
    let module = match Module::new(&config.store(), wasm_data) {
        Ok(module) => module,
        Err(err) => warn!("Failed to create module: {err}")
    };

    let env = WasmEnv::new(config.clone(), vec![]);
    let mut instance = match instance_from_module(module, config.store(), env) {
        Ok(instance) => instance,
        Err(err) => warn!("Failed to create instance: {err}")
    };

    let main = match instance
        .exports
        .get_typed_function::<i32, i32>(&instance.store, "arbitrum_main") {
        Ok(main) => main,
        Err(err) => warn!("Failed to get arbitrum_main: {err}")
    };
    let status = match main.call(&mut instance.store, 0) {
        Ok(status) => status,
        Err(err) => warn!("Failed to call arbitrum_main: {err}")
    };
    if status != 0 {
        warn!("Calling arbitrum_main returned non-zero: {status}")
    }

    // TODO: check instrumentation

    let counts = match instance.operator_counts() {
        Ok(counts) => counts,
        Err(err) => warn!("Failed to get operator counts: {err}")
    };
    for (op, count) in counts.into_iter() {
        println!("{op}\t{count}\n")
    }
});
