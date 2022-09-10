// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use crate::{depth::DepthChecker, meter::Meter};

use wasmer::{imports, CompilerConfig, Instance, Module, Store, Universal};
use wasmer_compiler_singlepass::Singlepass;
use eyre::Result;

use std::sync::Arc;

pub fn create(wasm: &[u8]) -> Result<Instance> {

    let mut compiler = Singlepass::new();
    compiler.canonicalize_nans(true);

    // add the instrumentation
    compiler.push_middleware(Arc::new(Meter::new()));
    compiler.push_middleware(Arc::new(DepthChecker::new()));

    let engine = Universal::new(compiler).engine();
    let store = Store::new(&engine);
    let module = Module::new(&store, wasm)?;
    let imports = imports! {};
    let instance = Instance::new(&module, &imports)?;
    Ok(instance)
}
