// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![no_main]
use common::color;
use libfuzzer_sys::fuzz_target;
use polyglot::{self, ExecPolyglot};
use prover::{
    middlewares::{depth::DepthCheckedMachine, meter::MeteredMachine},
    Machine,
};
use std::time::{Duration, Instant};

mod util;
mod wasm;

use util::{fail, fuzz_config, warn, wat};

fuzz_target!(|data: &[u8]| {
    let module = wasm::random(data, 1);
    if let Err(error) = polyglot::machine::validate(&module) {
        warn!("Failed to validate wasm {}", error);
    }

    let start = Instant::now();
    let config = fuzz_config();
    let mut machine = match Machine::from_polyglot_binary(&module, &config) {
        Ok(machine) => machine,
        Err(error) => {
            let error = error.to_string();
            let acceptable = vec![
                "Memory inits to a size larger",
                "Out-of-bounds data memory init",
                "Out of bounds element segment",
                "No implementation for floating point operation", // move to validate
            ];
            if acceptable.iter().any(|x| error.contains(x)) {
                warn!("Failed to create machine: {}", error);
            } else {
                fail!(module, "Failed to create machine: {}", error);
            }
        }
    };
    let machine_load_time = start.elapsed();

    let start = Instant::now();
    let mut instance = match polyglot::machine::create(&module, &config) {
        Ok(instance) => instance,
        Err(error) => fail!(module, "Failed to create instance: {}", error),
    };
    let instance_load_time = start.elapsed();

    let start = Instant::now();
    let instance_outcome = instance.execute();
    let instance_time = start.elapsed();

    let start = Instant::now();
    let machine_outcome = machine.execute();
    let machine_time = start.elapsed();

    let instance_gas = instance.gas_left();
    let machine_gas = machine.gas_left();

    let instance_space = instance.stack_space_left();
    let machine_space = machine.stack_space_left();

    macro_rules! maybe {
        ($value:expr, $other:expr) => {
            match $value == $other {
                true => format!("{}", $value),
                false => color::red(&$value),
            }
        };
    }

    macro_rules! check {
        ($value:expr, $other:expr) => {
            if $value != $other {
                println!("{}", color::red("Divergence"));
                println!(
                    "    Arbi: {} with {} and {} stack",
                    maybe!(machine_outcome, instance_outcome),
                    maybe!(machine_gas, instance_gas),
                    maybe!(machine_space, instance_space),
                );
                println!(
                    "    Poly: {} with {} and {} stack",
                    maybe!(instance_outcome, machine_outcome),
                    maybe!(instance_gas, machine_gas),
                    maybe!(instance_space, machine_space),
                );
                println!();
                fail!(module, "")
            }
        };
    }

    check!(instance_outcome, machine_outcome);
    check!(instance_gas, machine_gas);
    check!(instance_space, machine_space);

    if instance_outcome == polyglot::ExecOutcome::NoStart {
        warn!("no start");
    }

    println!(
        "Poly: {} with {} and {} stack",
        maybe!(&instance_outcome, &machine_outcome),
        maybe!(&instance_gas, &machine_gas),
        maybe!(&instance_space, &machine_space),
    );

    /*let gas: u64 = instance_gas.into();
    let count: u64 = config.start_gas - gas;
    let rate = 1_000_000.0 * count as f64 / instance_time as f64;

    let ratio = instance_time as f64 / machine_time as f64;*/
    println!(
        "Time {} {} {} {}",
        format_time(instance_load_time),
        format_time(instance_time),
        format_time(machine_load_time),
        format_time(machine_time),
    );
    println!();
});

fn format_time(span: Duration) -> String {
    let mut span = span.as_nanos() as f64;
    let mut unit = 0;
    let units = vec!["ns", "μs", "ms", "s"];
    let scale = vec![1000., 1000., 1000., 1000.];
    let colors = vec![color::MINT, color::MINT, color::YELLOW, color::RED];
    while span > 100. {
        span /= scale[unit];
        unit += 1;
    }
    color::color(
        colors[unit],
        format!("{:6}", format!("{:.1}{}", span, units[unit])),
    )
}
