// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use loupe::MemoryUsage;
use wasmer::{ExportIndex, FunctionMiddleware, LocalFunctionIndex, ModuleMiddleware};
use wasmer_types::ModuleInfo;

#[derive(Debug, MemoryUsage)]
pub struct StartMover {
    name: String,
}

impl StartMover {
    pub fn new(name: &str) -> Self {
        let name = name.to_owned();
        Self { name }
    }
}

impl ModuleMiddleware for StartMover {
    fn transform_module_info(&self, module: &mut ModuleInfo) {
        module.exports.remove(&self.name);

        if let Some(start) = module.start_function.take() {
            let export = ExportIndex::Function(start);
            module.exports.insert(self.name.clone(), export);
            module.function_names.insert(start, self.name.clone());
        }
    }

    fn generate_function_middleware<'a>(
        &self,
        _: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware<'a> + 'a> {
        Box::new(FunctionStartMover {})
    }
}

#[derive(Debug)]
struct FunctionStartMover {}

impl<'a> FunctionMiddleware<'a> for FunctionStartMover {}
