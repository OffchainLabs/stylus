// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![cfg(test)]

use crate::meter::Meter;

use wasmer::{imports, CompilerConfig, Instance, Module, Store, Universal, Value};
use wasmer_compiler_singlepass::Singlepass;

use eyre::Result;

use std::sync::Arc;

#[test]
fn test_fuel() -> Result<()> {
    let wasm = std::fs::read("../jit/programs/pure/main.wat")?;

    let mut compiler = Singlepass::new();
    compiler.canonicalize_nans(true);

    // add the instrumentation
    compiler.push_middleware(Arc::new(Meter::new()));

    let engine = Universal::new(compiler).engine();
    let store = Store::new(&engine);
    let module = Module::new(&store, &wasm)?;
    let imports = imports! {};
    let instance = Instance::new(&module, &imports)?;

    let add_one = instance.exports.get_function("add_one")?;
    let result = add_one.call(&[Value::I32(42)])?;
    assert_eq!(result[0], Value::I32(43));
    Ok(())
}
