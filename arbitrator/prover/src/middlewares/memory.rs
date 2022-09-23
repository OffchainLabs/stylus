// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use super::{DefaultFunctionMiddleware, Middleware, ModuleMod};

use eyre::Result;
use wasmer_types::{Bytes, LocalFunctionIndex, Pages};

use std::convert::TryFrom;

pub struct MemoryChecker {
    /// Upper bound on the amount of memory a module may use
    limit: Bytes,
}

impl MemoryChecker {
    pub fn new(limit: Bytes) -> Result<Self> {
        Pages::try_from(limit)?; // ensure limit isn't too large
        Ok(Self { limit })
    }
}

impl<'a> Middleware<'a> for MemoryChecker {
    type M = DefaultFunctionMiddleware;

    fn update_module(&self, module: &mut dyn ModuleMod) {
        let Bytes(table_bytes) = module.table_bytes();
        let Bytes(limit) = self.limit;
        let limit = limit.saturating_sub(table_bytes);
        let limit = Pages::try_from(Bytes(limit)).unwrap(); // checked in new()
        module.limit_memory(limit);
    }

    fn instrument(&self, _: LocalFunctionIndex) -> Self::M {
        DefaultFunctionMiddleware
    }
}
