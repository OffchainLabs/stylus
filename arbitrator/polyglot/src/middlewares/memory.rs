// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use std::convert::TryFrom;

use eyre::Result;
use loupe::MemoryUsage;
use wasmer::{FunctionMiddleware, LocalFunctionIndex, ModuleMiddleware};
use wasmer_types::{Bytes, ModuleInfo, Pages};

#[derive(Debug, MemoryUsage)]
pub struct MemoryChecker {
    /// Upper bound on the amount of memory a module may use, measured in 64kb pages
    limit: usize,
}

impl MemoryChecker {
    pub fn new(limit: usize) -> Result<Self> {
        Pages::try_from(Bytes(limit))?; // ensure limit isn't too large
        Ok(Self { limit })
    }
}

impl ModuleMiddleware for MemoryChecker {
    fn transform_module_info(&self, module: &mut ModuleInfo) {
        let mut reserved = 0;
        for (_, table) in &module.tables {
            // We don't support `TableGrow`, so the minimum is the size a table will always be.
            // We also don't support the 128-bit extension, so we'll say a `type` is at most 8 bytes.
            reserved += 8 * table.minimum;
        }

        // a zero limit will induce an error
        let limit = self.limit.saturating_sub(reserved as usize);
        let limit = Pages::try_from(Bytes(limit)).unwrap();

        for (_, memory) in &mut module.memories {
            let limit = memory.maximum.unwrap_or(limit);
            let pages = limit.min(limit);
            memory.maximum = Some(pages);
        }
    }

    fn generate_function_middleware<'a>(
        &self,
        _: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware<'a> + 'a> {
        Box::new(FunctionMemoryChecker {})
    }
}

#[derive(Debug)]
struct FunctionMemoryChecker {}

impl<'a> FunctionMiddleware<'a> for FunctionMemoryChecker {}
