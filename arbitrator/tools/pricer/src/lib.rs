// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

pub mod wasm;

#[macro_export]
macro_rules! wat {
    ($wasm:expr) => {{
        let text = wasmprinter::print_bytes(&$wasm);
        text.map_err(|x| eyre::eyre!(x))
    }};
}

#[macro_export]
macro_rules! fail {
    ($wasm:expr, $form:expr $(,$args:expr)*) => {{
        println!("{}", "Failing case".red());
        println!("{}", wat!($wasm));

        let message = format!($form $(,$args.debug_red())*);
        if message != "" {
            eprintln!("{message}");
        }
        panic!("Fatal error");
    }}
}
