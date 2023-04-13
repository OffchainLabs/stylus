// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

pub mod wasm;

#[macro_export]
macro_rules! wat {
    ($wasm:expr) => {
        wabt::Wasm2Wat::new()
            .fold_exprs(true)
            .inline_export(true)
            .convert($wasm)
            .map(|buf| String::from_utf8(buf.as_ref().to_vec()).unwrap())
            .unwrap_or("???".to_string())
        //.as_ref()
        //to_vec();
        //String::from_utf8(wat).unwrap()
    };
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

//pub use {fail, wat};
