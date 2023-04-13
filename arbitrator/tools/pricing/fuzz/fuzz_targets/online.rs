// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![no_main]
use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
    time::Instant, fs::File,
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
    config.start_ink = 1_000_000;
    config.costs = |_| 1;

    let mut native = match NativeInstance::from_bytes(&module, &config) {
        Ok(native) => native,
        //Err(error) => fail!(module, "Failed to create instance: {}", error),
        Err(_) => return,
    };
    time!("Init", 250_000);

    let cycles = unsafe { core::arch::x86_64::_rdtsc() };
    start = Instant::now();
    let starter = native.get_start().expect("no start");
    let outcome = starter.call(&mut native.store);
    /*if let Err(_error) =  {
        //fail!(module, "Execution error: {}", error)
    }*/
    time!("Exec", 2_000);
    let after = unsafe { core::arch::x86_64::_rdtsc() };
    let elapsed = after - cycles;

    let counts = native.operator_counts().unwrap();
    if counts.len() == 0 {
        return
    }
    //let counts: Vec<_> = counts.into_iter().map(|(k, v)| (k, v as f64)).collect();

    let mut weights = WEIGHTS.lock();

    let mut predicted = weights.start_weight;
    match outcome.is_ok() {
        true => predicted += weights.success_weight,
        false => predicted += weights.error_weight,
    }
    for (&op, &count) in &counts {
        predicted += *weights.ops.entry(op).or_insert(1.) * count as f64;
    }
    let observed = elapsed as f64;

    let alpha = 0.001;
    let delta = 1.0 + (observed - predicted) / predicted;
    macro_rules! lerp {
        ($curr:expr, $new:expr, $factor:expr) => {
            $factor * $new + (1. - $factor) * $curr
        };
    }
    macro_rules! propagate {
        ($count:expr, $weight:expr) => {{
            if $count != 0 {
                let curr = *$weight;
                let prop = $count as f64 / counts.len() as f64;
                //let mut update = lerp!(curr, curr * delta, alpha / $count as f64);
                let mut update = lerp!(curr, curr * delta, prop * alpha);
                if update <= 1. {
                    update = 1.;
                }
                if update.is_nan() || update.is_infinite() {
                    println!("Invalid update {} {} {}", curr, update, predicted);
                    return;
                }
                *$weight = update;
            }
        }};
    }

    propagate!(1, &mut weights.start_weight);
    match outcome.is_ok() {
        true => propagate!(1, &mut weights.success_weight),
        false => propagate!(1, &mut weights.error_weight),
    }
    for (op, &count) in &counts {
        propagate!(count, weights.ops.get_mut(op).unwrap());
    }
    weights.updates += 1;

    if weights.updates % 1_000 != 1 {
        return;
    }

    println!(
        "{} {} {}",
        "Update".blue(),
        weights.updates.blue(),
        counts.len()
    );
    println!(
        "Delta {:.0} - {:.0} = {:.0} => {:.2}%",
        observed,
        predicted,
        observed - predicted,
        delta * 100. - 100.
    );
    println!(
        "Meta {:.0} {:.0} {:.0}",
        weights.start_weight, weights.success_weight, weights.error_weight
    );
    /*for (op, count) in counts {
        let weight = weights.ops.get(&op).unwrap();
        println!("{} {:.1} {}", op, weight, count.grey());
    }*/

    let mut sorted: Vec<_> = weights.ops.clone().into_iter().collect();
    sorted.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (op, weight) in &sorted {
        println!("{} {:.2}", op, weight)
    }
    let ops_ran: u64 = counts.iter().map(|x| x.1).sum();
    println!("Wat {} {} {}", counts.len(), ops_ran, wat!(module));

    let mut file = File::create("data.csv").unwrap();
    writeln!(file, "MetaStart\t{}", weights.start_weight).unwrap();
    writeln!(file, "MetaFinish\t{}", weights.success_weight).unwrap();
    writeln!(file, "MetaTrap\t{}", weights.error_weight).unwrap();
    for (op, weight) in sorted {
        writeln!(file, "{}\t{:.2}", op, weight).unwrap();
    }
});

#[derive(Default)]
struct Weights {
    ops: HashMap<OperatorCode, f64>,
    start_weight: f64,
    error_weight: f64,
    success_weight: f64,
    updates: usize,
}

lazy_static! {
    static ref WEIGHTS: Mutex<Weights> = {
        let mut weights = Weights::default();
        weights.start_weight = 1.;
        weights.error_weight = 1.;
        weights.success_weight = 1.;
        Mutex::new(weights)
    };
}
