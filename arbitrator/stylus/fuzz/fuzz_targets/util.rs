// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![allow(unused_macros)]
macro_rules! warn_ {
    ($text:expr $(,$args:expr)*) => {{
        eprintln!($text $(,$args.red())*);
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
        println!("{}", "Failing case".red());
        println!("{}", wat!($wasm));

        let message = format!($form $(,$args.red())*);
        if message != "" {
            eprintln!("{message}");
        }
        panic!("Fatal error");
    }}
}

pub(crate) use {fail, warn_ as warn, wat};
