// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![cfg(test)]

use crate::machine;

use eyre::Result;
use wasmer::Value;

#[test]
fn test_gas() -> Result<()> {
    let wasm = std::fs::read("../jit/programs/pure/main.wat")?;
    let instance = machine::create(&wasm)?;

    let add_one = instance.exports.get_function("add_one")?;
    let result = add_one.call(&[Value::I32(42)])?;
    assert_eq!(result[0], Value::I32(43));
    Ok(())
}
