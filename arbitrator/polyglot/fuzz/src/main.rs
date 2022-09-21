// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use honggfuzz::fuzz;
use polyglot;
use wasmparser::Operator;

mod wasm;

fn main() {
    let costs = |_: &Operator| 0;

    loop {
        fuzz!(|data: &[u8]| {
            let module = wasm::random(data);

            if let Err(error) = polyglot::util::validate(&module) {
                eprintln!("Failed to validate wasm {error}");
                return;
            }

            let _instance = match polyglot::machine::create(&module, costs, 64) {
                Ok(instance) => instance,
                Err(error) => {
                    eprintln!("Failed {error}");
                    return;
                }
            };
        });
    }
}
