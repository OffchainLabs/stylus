// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use std::mem;

use loupe::{MemoryUsage, MemoryUsageTracker};
use parking_lot::Mutex;
use wasmer::{
    ExportIndex, FunctionMiddleware, GlobalInit, GlobalType, LocalFunctionIndex, MiddlewareError,
    MiddlewareReaderState, ModuleMiddleware, Mutability, Type,
};
use wasmer_types::{GlobalIndex, ModuleInfo};
use wasmparser::Operator;

use crate::meter::add_global;

#[derive(Debug)]
pub struct DepthChecker {
    global: Mutex<Option<GlobalIndex>>,
}

impl DepthChecker {
    pub fn new() -> Self {
        Self {
            global: Mutex::new(None),
        }
    }
}

impl MemoryUsage for DepthChecker {
    fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + mem::size_of::<Option<GlobalIndex>>()
    }
}

impl ModuleMiddleware for DepthChecker {
    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let depth = add_global(module_info, "stack_depth");
        let global = &mut *self.global.lock();
        assert_eq!(*global, None, "meter already set");
        *global = Some(depth);
    }

    fn generate_function_middleware(&self, _: LocalFunctionIndex) -> Box<dyn FunctionMiddleware> {
        let global = self.global.lock().expect("no global");
        Box::new(FunctionDepthChecker::new(global))
    }
}

#[derive(Debug)]
struct FunctionDepthChecker {
    global: GlobalIndex,
}

impl FunctionDepthChecker {
    fn new(global: GlobalIndex) -> Self {
        Self { global }
    }
}

impl FunctionMiddleware for FunctionDepthChecker {
    fn feed<'a>(
        &mut self,
        operator: Operator<'a>,
        state: &mut MiddlewareReaderState<'a>,
    ) -> Result<(), MiddlewareError> {
        state.push_operator(operator);
        Ok(())
    }
}
