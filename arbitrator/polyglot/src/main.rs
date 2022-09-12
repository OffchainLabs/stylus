// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use wasmparser::Operator;

mod depth;
mod machine;
mod meter;
mod test;
mod util;

fn main() {
    let costs = |_: &Operator| 1;

    let wasm = std::fs::read("../jit/programs/pure/main.wat").unwrap();
    machine::create(&wasm, costs, 1024).expect("failed to create machine");
    println!("Hello, world!");
}
