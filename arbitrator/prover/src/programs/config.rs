// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/

use wasmer_types::{Bytes, GlobalIndex};
use wasmparser::Operator;

use parking_lot::Mutex;

use std::collections::HashMap;

#[cfg(feature = "native")]
use {
    super::{
        counter::Counter, depth::DepthChecker, memory::MemoryChecker, meter::Meter, start::StartMover,
        WasmerMiddlewareWrapper,
    },
    eyre::Result,
    std::sync::Arc,
    wasmer::{CompilerConfig, Store, Universal},
    wasmer_compiler_singlepass::Singlepass,
};

pub struct PolyglotConfig {
    pub costs: fn(&Operator) -> u64,
    pub start_gas: u64,
    pub max_depth: u32,
    pub memory_limit: Bytes,
    pub opcode_counts_global_indexes: Option<Arc<Mutex<Vec<GlobalIndex>>>>,
    pub operator_code_to_count_index: Option<Arc<Mutex<HashMap<usize, usize>>>>,
}

impl Default for PolyglotConfig {
    fn default() -> Self {
        let costs = |_: &Operator| 0;
        Self {
            costs,
            start_gas: 0,
            max_depth: 1024,
            memory_limit: Bytes(2 * 1024 * 1024),
            opcode_counts_global_indexes: None,
            operator_code_to_count_index: None,
        }
    }
}

#[cfg(feature = "native")]
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
        if self.opcode_counts_global_indexes.is_some() && self.operator_code_to_count_index.is_some() {
            let counter = WasmerMiddlewareWrapper::new(Counter::new(self.opcode_counts_global_indexes.as_ref().unwrap().clone(), self.operator_code_to_count_index.as_ref().unwrap().clone()));
            compiler.push_middleware(Arc::new(counter));
        }
        compiler.push_middleware(Arc::new(start));

        let engine = Universal::new(compiler).engine();
        Ok(Store::new(&engine))
    }
}
