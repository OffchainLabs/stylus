// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![no_main]

use arbutil::color;
use libfuzzer_sys::fuzz_target;
use parking_lot::Mutex;
use polyglot::{self, ExecOutcome, ExecPolyglot, machine};
use prover::programs::{
    depth::DepthCheckedMachine,
    meter::{MachineMeter, MeteredMachine},
};

mod util;
mod wasm;

use util::{fail, fuzz_config, warn, wat};
use std::{collections::HashMap, sync::Arc};

fuzz_target!(|data: &[u8]| {
    let module = wasm::random(data, 0);
    if let Err(error) = polyglot::machine::validate(&module) {
        warn!("Failed to validate wasm {}", error);
    }

    //warn!("{}", wat!(module));

    let (config, env) = fuzz_config();
    //config.opcode_counts_global_indexes = Some(Arc::new(Mutex::new(Vec::new())));
    //config.operator_code_to_count_index = Some(Arc::new(Mutex::new(HashMap::new())));
    let (module, store) = machine::instrument(&module, &config).unwrap();
    let mut instance = match polyglot::machine::create(&module, &store, env) {
        Ok(instance) => instance,
        Err(error) => warn!("Failed to create instance: {}", error),
    };

    let outcome = instance.run_start();
    let space = instance.stack_space_left();
    let gas = match instance.gas_left() {
        MachineMeter::Ready(gas) => gas,
        MachineMeter::Exhausted => warn!("Call failed: {}", "Out of gas"),
    };

    use ExecOutcome::*;
    match outcome {
        NoStart => {}
        Success(_) => {}
        Revert(output) => warn!("reverted with {}", hex::encode(output)),
        Failure(error) => warn!(
            "Call failed with {} words and {} gas left: {}",
            space, gas, error
        ),
        OutOfGas => warn!("Call failed: {}", "Out of gas"),
        OutOfStack => warn!("Call failed: {}", "Out of stack"),
        StackOverflow => fail!(
            module,
            "Fatal: {} {} words and {} gas left",
            "stack overflow",
            space,
            gas
        ),
        FatalError(error) => fail!(
            module,
            "Fatal: {} {} words and {} gas left",
            error,
            space,
            gas
        )
    }
});
