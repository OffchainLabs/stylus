// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![no_main]
use common::color;
use libfuzzer_sys::fuzz_target;
use polyglot::{self, MachineMeter};
use wasmparser::Operator;

mod wasm;

fuzz_target!(|data: &[u8]| {
    macro_rules! warn {
        ($text:expr $(,$args:expr)*) => {{
            eprintln!($text $(,color::red($args))*);
            return;
        }}
    }

    let module = wasm::random(data);
    if let Err(error) = polyglot::machine::validate(&module) {
        warn!("Failed to validate wasm {}", error);
    }

    macro_rules! fail {
        ($form:expr $(,$args:expr)*) => {{
            let wat = wabt::Wasm2Wat::new()
                .fold_exprs(true)
                .inline_export(true)
                .convert(module)
                .expect("wasm2wat failure")
                .as_ref()
                .to_vec();
            let text = String::from_utf8(wat).unwrap();
            println!("{text}");

            let message = format!($form $(,color::red($args))*);
            eprintln!("{message}");
            panic!("Fatal error");
        }}
    }

    let stack = 64 * 1024;
    let costs = |_: &Operator| 1;
    let instance = match polyglot::machine::create(&module, costs, 128 * 1024, stack) {
        Ok(instance) => instance,
        Err(error) => warn!("Failed to create instance: {}", error),
    };

    let start = match instance.exports.get_function("polyglot_moved_start").ok() {
        Some(start) => start.native::<(), ()>().unwrap(),
        None => return,
    };

    if let Err(error) = start.call() {
        let gas = match polyglot::meter::gas_left(&instance) {
            MachineMeter::Ready(gas) => gas,
            MachineMeter::Exhausted => warn!("Call failed: {}", "Out of gas"),
        };

        let left = match polyglot::depth::stack_space_remaining(&instance) {
            0 => warn!("Call failed: {}", "Out of stack"),
            left => left,
        };

        if error
            .to_string()
            .contains("RuntimeError: call stack exhausted")
        {
            fail!(
                "Fatal: {} {} words left with {} gas left",
                "stack overflow",
                left,
                gas
            )
        }

        warn!(
            "Call failed with {} words and {} gas left: {}",
            left, gas, error
        );
    }
});
