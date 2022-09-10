// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

mod depth;
mod machine;
mod meter;
mod test;

fn main() {
    let wasm = std::fs::read("../jit/programs/pure/main.wat").unwrap();
    machine::create(&wasm).expect("failed to create machine");
    println!("Hello, world!");
}
