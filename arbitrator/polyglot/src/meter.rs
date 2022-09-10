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

#[derive(Debug)]
pub struct Meter {
    global: Mutex<Option<GlobalIndex>>,
}

impl Meter {
    pub fn new() -> Self {
        Self {
            global: Mutex::new(None),
        }
    }
}

impl MemoryUsage for Meter {
    fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + mem::size_of::<Option<GlobalIndex>>()
    }
}

impl ModuleMiddleware for Meter {
    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let gas = add_global(module_info, "fuel_left");
        let global = &mut *self.global.lock();
        assert_eq!(*global, None, "meter already set");
        *global = Some(gas);
    }

    fn generate_function_middleware(&self, _: LocalFunctionIndex) -> Box<dyn FunctionMiddleware> {
        let global = self.global.lock().expect("no global");
        Box::new(FunctionMeter::new(global))
    }
}

pub fn add_global(module: &mut ModuleInfo, name: &str) -> GlobalIndex {
    let name = format!("polyglot_{name}");
    let global_type = GlobalType::new(Type::I32, Mutability::Var);

    let index = module.globals.push(global_type);
    module.exports.insert(name, ExportIndex::Global(index));
    module.global_initializers.push(GlobalInit::I32Const(0));
    index
}

#[derive(Debug)]
struct FunctionMeter {
    global: GlobalIndex,
    block_cost: usize,
}

impl FunctionMeter {
    fn new(global: GlobalIndex) -> Self {
        let block_cost = 0;
        Self { global, block_cost }
    }
}

impl FunctionMiddleware for FunctionMeter {
    fn feed<'a>(
        &mut self,
        operator: Operator<'a>,
        state: &mut MiddlewareReaderState<'a>,
    ) -> Result<(), MiddlewareError> {

        self.block_cost += operator_cost(&operator);

        use Operator::*;
        let end = match operator {
            _ => true,
        };
        
        state.push_operator(operator);
        Ok(())
    }
}

fn operator_cost(operator: &Operator<'_>) -> usize {
    match operator {
        _ => 1
    }
}
