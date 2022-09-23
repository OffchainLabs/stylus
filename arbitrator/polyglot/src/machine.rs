// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use prover::middlewares::{
    depth::DepthChecker, memory::MemoryChecker, meter::Meter, start::StartMover,
    WasmerMiddlewareWrapper,
};

use eyre::Result;
use wasmer::{imports, CompilerConfig, Instance, Module, Store, Universal};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_types::Bytes;
use wasmparser::Operator;

use std::sync::Arc;

pub fn create(
    wasm: &[u8],
    costs: fn(&Operator) -> u64,
    start_gas: u64,
    max_depth: u32,
) -> Result<Instance> {
    let mut compiler = Singlepass::new();
    compiler.canonicalize_nans(true);
    compiler.enable_verifier();

    let meter = WasmerMiddlewareWrapper::new(Meter::new(costs, start_gas));
    let depth = WasmerMiddlewareWrapper::new(DepthChecker::new(max_depth));
    let memory = WasmerMiddlewareWrapper::new(MemoryChecker::new(Bytes(1024 * 1024))?); // 1 MB memory limit
    let start = WasmerMiddlewareWrapper::new(StartMover::new("polyglot_moved_start"));

    // add the instrumentation
    compiler.push_middleware(Arc::new(meter));
    compiler.push_middleware(Arc::new(depth));
    compiler.push_middleware(Arc::new(memory));
    compiler.push_middleware(Arc::new(start));

    let engine = Universal::new(compiler).engine();
    let store = Store::new(&engine);
    let module = Module::new(&store, wasm)?;
    let imports = imports! {};
    let instance = Instance::new(&module, &imports)?;
    Ok(instance)
}

pub fn validate(wasm: &[u8]) -> Result<()> {
    let features = wasmparser::WasmFeatures {
        mutable_global: true,
        saturating_float_to_int: true,
        sign_extension: true,
        reference_types: false,
        multi_value: true,
        bulk_memory: false,
        module_linking: false,
        simd: false,
        relaxed_simd: false,
        threads: false,
        tail_call: false,
        deterministic_only: false,
        multi_memory: false,
        exceptions: false,
        memory64: false,
        extended_const: false,
        //component_model: false, TODO: add in 0.84
    };
    let mut validator = wasmparser::Validator::new();
    validator.wasm_features(features);
    validator.validate_all(wasm)?;
    Ok(())
}
