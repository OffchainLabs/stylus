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

mod util;
mod wasm;

use util::{fail, fuzz_config, warn, wat};

fuzz_target!(|data: &[u8]| {
    let module = wasm::random(data, 1);
    if let Err(error) = polyglot::machine::validate(&module) {
        warn!("Failed to validate wasm {}", error);
    }

    let config = fuzz_config();
    let mut machine = match Machine::from_polyglot_binary(&module, &config) {
        Ok(machine) => machine,
        Err(error) => {
            let error = error.to_string();
            if error.contains("Memory inits to a size larger") {
                warn!("Failed to create machine: {}", error);
            } else {
                fail!(module, "Failed to create machine: {}", error);
            }
        }
    };

    let mut instance = match polyglot::machine::create(&module, &config) {
        Ok(instance) => instance,
        Err(error) => fail!(module, "Failed to create instance: {}", error),
    };

    let instance_outcome = instance.execute();
    let machine_outcome = machine.execute();

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
});
