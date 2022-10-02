// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use polyglot::machine::WasmEnvArc;
use prover::middlewares::PolyglotConfig;
use wasmer::wasmparser::Operator;

pub fn fuzz_config() -> (PolyglotConfig, WasmEnvArc) {
    let env = WasmEnvArc::new(&[], 1000);
    let mut config = PolyglotConfig::default();
    config.costs = |op: &Operator| match op {
        Operator::BrTable { table } => {
            2 + 4 * table.targets().size_hint().0 as u64
        },
        Operator::LocalTee { .. } => 2,
        _ => 1,
    };
    config.start_gas = 128 * 1024;
    config.max_depth = 64 * 1024;
    (config, env)
}

macro_rules! warn_ {
    ($text:expr $(,$args:expr)*) => {{
        eprintln!($text $(,color::red($args))*);
        return;
    }}
}

macro_rules! wat {
    ($wasm:expr) => {{
        let wat = wabt::Wasm2Wat::new()
            .fold_exprs(true)
            .inline_export(true)
            .convert($wasm);
        match wat {
            Ok(wat) => String::from_utf8(wat.as_ref().to_vec()).unwrap(),
            Err(err) => format!("wasm2wat failed: {}", err),
        }
    }};
}

macro_rules! fail {
    ($wasm:expr, $form:expr $(,$args:expr)*) => {{
        println!("{}", color::red("Failing case"));
        println!("{}", wat!($wasm));

        let message = format!($form $(,color::red($args))*);
        if message != "" {
            eprintln!("{message}");
        }
        panic!("Fatal error");
    }}
}

pub(crate) use {fail, warn_ as warn, wat};
