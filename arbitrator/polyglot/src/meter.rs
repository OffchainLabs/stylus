// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use loupe::{MemoryUsage, MemoryUsageTracker};
use wasmer::{
    FunctionMiddleware, LocalFunctionIndex, MiddlewareError, MiddlewareReaderState,
    ModuleMiddleware,
};
use wasmparser::Operator;

#[derive(Debug)]
pub struct Meter {}

impl Meter {
    pub fn new() -> Self {
        Self {}
    }
}

impl MemoryUsage for Meter {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        0
    }
}

impl ModuleMiddleware for Meter {
    fn generate_function_middleware(
        &self,
        local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FuncMeter {})
    }
}

#[derive(Debug)]
struct FuncMeter {}

impl FunctionMiddleware for FuncMeter {
    fn feed<'a>(
        &mut self,
        operator: Operator<'a>,
        state: &mut MiddlewareReaderState<'a>,
    ) -> Result<(), MiddlewareError> {
        state.push_operator(operator);
        Ok(())
    }
}
