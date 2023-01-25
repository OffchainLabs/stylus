// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![allow(
    clippy::field_reassign_with_default,
    clippy::inconsistent_digit_grouping
)]

use crate::stylus::instance_from_module;
use crate::{
    env::WasmEnv,
    run::RunProgram,
    stylus::{self, NativeInstance},
};
use arbutil::{crypto, Color};
use eyre::{bail, Result};
use prover::programs::config::{StylusConfig, StylusDebugConfig};
use prover::{
    binary,
    programs::{
        counter::{Counter, CountingMachine},
        prelude::*,
        start::StartMover,
        MiddlewareWrapper, ModuleMod, STYLUS_ENTRY_POINT,
    },
    Machine,
};
use std::{path::Path, sync::Arc};
use wasmer::wasmparser::Operator;
use wasmer::{
    imports, CompilerConfig, ExportIndex, Function, Imports, Instance, MemoryType, Module, Pages,
    Store,
};
use wasmer_compiler_singlepass::Singlepass;

mod wasm;

#[test]
fn test_fuzz() -> Result<()> {
    let mut config = StylusConfig::default();
    config.debug = Some(StylusDebugConfig::default());

    let data = vec![92, 3, 0, 0, 0, 32, 251, 115, 116, 121, 108, 117, 115, 95, 111, 112, 99, 111, 100, 101, 54, 55, 95, 99, 111, 117, 110, 116, 65];
    let wasm_data = wasm::random(&data, 0);
    let module = match Module::new(&config.store(), wasm_data) {
        Ok(module) => module,
        Err(err) => {
            panic!("Failed to create instance: {err}");
            //return Ok(());
        }
    };

    let env = WasmEnv::new(config.clone(), vec![]);
    let mut instance = match instance_from_module(module, config.store(), env) {
        Ok(instance) => instance,
        Err(err) => {
            if err.to_string().eq("Missing export memory") {
                return Ok(());
            }
            println!("Failed to create instance: {err}");
            panic!("Failed to create instance: {err}")
        }
    };

    println!("Getting main");
    let main = match instance
        .exports
        .get_typed_function::<i32, i32>(&instance.store, "arbitrum_main")
    {
        Ok(main) => main,
        Err(err) => {
            println!("Failed to get arbitrum_main: {err}");
            return Ok(());
        }
    };
    println!("Calling main");
    let status = match main.call(&mut instance.store, 0) {
        Ok(status) => status,
        Err(err) => {
            println!("Failed to call arbitrum_main: {err}");
            return Ok(());
        }
    };
    if status != 0 {
        println!("Calling arbitrum_main returned non-zero: {status}")
    }
    println!("Finished main");

    // TODO: check instrumentation

    let counts = match instance.operator_counts() {
        Ok(counts) => counts,
        Err(err) => {
            println!("Failed to get operator counts: {err}");
            return Ok(());
        }
    };
    for (op, count) in counts.into_iter() {
        println!("{op}\t{count}\n")
    }

    Ok(())
}
