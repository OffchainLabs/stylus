// Copyright 2021-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![no_main]
#![allow(clippy::field_reassign_with_default, unused_imports)]


mod util;
mod wasm;

use regex;
use std::sync::atomic::{AtomicU32, Ordering};
use libfuzzer_sys::{Corpus, fuzz_target};
use prover::{binary::parse, programs::{start::StartlessMachine, config::{StylusConfig, StylusDebugConfig}, counter::CountingMachine}};
use std::path::Path;
use stylus::{env::WasmEnv, stylus::{instance_from_module, NativeInstance}};
use util::{wat, warn, fail};
use wasmer::Module;
use wasmparser::Operator;

static RUNS_EXECUTED: AtomicU32 = AtomicU32::new(0);

fn rewrite_memory_line(line: String) -> String {
    let prefix = "(memory (;0;) (export \"";
    if !line.starts_with(prefix) {
        println!("nonconforming line: .{}.", line);
        panic!();
    } else {
        let re = regex::Regex::new("\"[^\"]+\"").expect("static");
        return re.replace(&line, "\"memory\"").to_string();
    }
}

fn modify_wat(wat: String) -> String {
    let mut out = vec![];
    let mut count = 0;
    for line in wat.lines() {
        let line = line.trim().to_string();
        println!("line {} was .{}.", count, line);
        if line.starts_with("(memory") {
            let line = rewrite_memory_line(line);
            println!("line was modified to .{}", line);
            out.push(line);
        } else {
            out.push(line);
        }
        count += 1;
    }
    let ans = out.join("\n");
    println!("ans: {}", ans);
    ans
}

fn fuzz_me(data: &[u8]) -> Corpus {
    let wasm_data  = wasm::random(data, 0);
    // dbg!(&wasm_data);
    let wat = match wabt::Wasm2Wat::new()
                .fold_exprs(true)
                .inline_export(true)
                .convert(&wasm_data)
        {
            Ok(wat) => String::from_utf8(wat.as_ref().to_vec()).unwrap(),
            Err(err) => format!("wasm2wat failed: {}", err),
        };
    let new_wat = modify_wat(wat);
    let wasm_data = wabt::wat2wasm(new_wat).unwrap();
    // let round_trip_wasm = wabt::wat2wasm(&wat).unwrap();
    // if wasm_data != round_trip_wasm {
    //     let wat2 = match wabt::Wasm2Wat::new()
    //     .fold_exprs(true)
    //     .inline_export(true)
    //     .convert(&wasm_data)
    //     {
    //         Ok(wat) => String::from_utf8(wat.as_ref().to_vec()).unwrap(),
    //         Err(err) => format!("wasm2wat failed: {}", err),
    //     };
    //     println!("fst: {} \nsnd: {}", wat, wat2);
    //     panic!("wats differed");
    // }

    let cur_runs = RUNS_EXECUTED.load(Ordering::Relaxed);
    let to_print = cur_runs % 100 == 0;
    if to_print {
        println!("run = {}; bytes_len = {}' wat = {}", cur_runs, data.len(), wat!(&wasm_data));
    }
    RUNS_EXECUTED.store(cur_runs+1, Ordering::Relaxed);

    if !wasm::validate(&wasm_data) {
        return Corpus::Reject;
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
        Err(err) => {
            println!("error in module creation: {}", err);
            return Corpus::Reject;
        }
    };
    // println!("hi1");
    let env = WasmEnv::new(config.clone(), vec![]);
    let mut instance = match instance_from_module(module, config.store(), env) {
        Ok(instance) => instance,
        Err(err) => {
            let err = err.to_string();

            if err.contains("Missing export memory") 
                || err.contains("out of bounds memory access") 
                || err.contains("out of bounds table access") 
            //    err.contains("Incompatible Export Type") ||
            //    err.contains("WebAssembly transaction error") ||
            {
                if to_print {
                    println!("err: {}", err)
                }
                return Corpus::Reject;
            }
            dbg!(&err);
            println!("data = {:?}", &data);
            println!("wat = {}", wat!(&wasm_data));
            // println!("{}", wat!(&wasm_data));
            panic!("Failed to create instance: {}", err);
        }
    };
    println!("data = {:?}", &data);
    println!("wat = {}", wat!(&wasm_data));
    println!("hi2");
    let starter = match instance.get_start() {
        Ok(starter) => starter,
        Err(err) => {
            println!("{}", wat!(&wasm_data));
            // panic!("Failed to get start: {err}");
            println!("Failed to get start: {err}");
            return Corpus::Reject;
        }
    };
    println!("hi3");
    if let Err(e) = starter.call(&mut instance.store) {
        println!("{}", wat!(&wasm_data));
        println!("Failed to run start: {e}");
        return Corpus::Reject;
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
}

fuzz_target!(|data: &[u8]| -> Corpus {
    fuzz_me(data)
});
