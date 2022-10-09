// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/

use super::{
    depth::DepthChecker, memory::MemoryChecker, meter::Meter, start::StartMover,
    WasmerMiddlewareWrapper,
};
use eyre::Result;
use std::sync::Arc;
use wasmer::{wasmparser::Operator, CompilerConfig, Store, Universal};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_types::Bytes;

pub struct PolyglotConfig {
    pub costs: fn(&Operator) -> u64,
    pub start_gas: u64,
    pub max_depth: u32,
    pub memory_limit: Bytes,
}

impl Default for PolyglotConfig {
    fn default() -> Self {
        let costs = |_: &Operator| 0;
        Self {
            costs,
            start_gas: 0,
            max_depth: 1024,
            memory_limit: Bytes(2 * 1024 * 1024),
        }
    }
}

impl PolyglotConfig {
    pub fn store(&self) -> Result<Store> {
        let mut compiler = Singlepass::new();
        compiler.canonicalize_nans(true);
        compiler.enable_verifier();

        let meter = WasmerMiddlewareWrapper::new(Meter::new(self.costs, self.start_gas));
        let depth = WasmerMiddlewareWrapper::new(DepthChecker::new(self.max_depth));
        let memory = WasmerMiddlewareWrapper::new(MemoryChecker::new(self.memory_limit)?);
        let start = WasmerMiddlewareWrapper::new(StartMover::new("polyglot_moved_start"));

        // add the instrumentation
        compiler.push_middleware(Arc::new(meter));
        compiler.push_middleware(Arc::new(depth));
        compiler.push_middleware(Arc::new(memory));
        compiler.push_middleware(Arc::new(start));

        let engine = Universal::new(compiler).engine();
        Ok(Store::new(&engine))
    }
}
