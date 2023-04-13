// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![no_main]
use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
    time::Instant, fs::{File, OpenOptions},
};

use arbutil::{format, operator::OperatorCode, Color};
use lazy_static::lazy_static;
use libfuzzer_sys::fuzz_target;
use parking_lot::Mutex;
use pricing::{fail, wasm, wat};
use prover::programs::{counter::CountingMachine, prelude::*};
use stylus::native::NativeInstance;

fuzz_target!(|data: &[u8]| {
    let module = wasm::random(data, 1);
    if let Err(_error) = wasm::validate(&module) {
        //warn!("Failed to validate wasm {}", error);
        return;
    }

    let mut start = Instant::now();

    macro_rules! time {
        ($name:expr, $threshold:expr) => {{
            let elapsed = start.elapsed();
            if elapsed.as_micros() > $threshold {
                println!("{} {}", $name.grey(), format::time(elapsed));
            }
            elapsed
        }};
    }

    let mut config = StylusConfig::version(1);
    config.debug.count_ops = true;
    config.debug.debug_funcs = true;
    config.pricing.ink_price = 100_00;
    config.start_ink = 500_000;
    config.costs = |_| 1;

    let mut native = match NativeInstance::from_bytes(&module, &config) {
        Ok(native) => native,
        //Err(error) => fail!(module, "Failed to create instance: {}", error),
        Err(_) => return,
    };
    time!("Init", 250_000);

    let mut aux = 0_u32;
    let before = unsafe { core::arch::x86_64::__rdtscp(&mut aux as *mut _) };
    start = Instant::now();
    let starter = native.get_start().expect("no start");
    let outcome = starter.call(&mut native.store).is_ok();
    time!("Exec", 250_000);
    
    let cycles = unsafe { core::arch::x86_64::__rdtscp(&mut aux as *mut _) - before };

    let counts = native.operator_counts().unwrap();
    if counts.len() == 0 {
        return
    }

    let outcome = outcome.then_some(1).unwrap_or_default();

    let mut file = OpenOptions::new().create(true).write(true).append(true).open("data.csv").unwrap();
    write!(file, "{cycles} {outcome} ").unwrap();
    for (op, count) in counts {
        write!(file, "{} {} ", op.0, count).unwrap();
    }
    writeln!(file).unwrap();
});
