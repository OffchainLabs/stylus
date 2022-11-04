// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use super::{DefaultFunctionMiddleware, Middleware, ModuleMod, TransformError};

use eyre::Result;
use loupe::MemoryUsage;
use wasmer_types::{Bytes, LocalFunctionIndex, Pages};

use std::{convert::TryFrom, mem};

#[derive(Debug)]
pub struct MemoryChecker {
    /// Upper bound on the amount of memory a module may use
    limit: Bytes,
}

impl MemoryUsage for MemoryChecker {
    fn size_of_val(&self, _: &mut dyn loupe::MemoryUsageTracker) -> usize {
        mem::size_of::<Bytes>()
    }
}

impl MemoryChecker {
    pub fn new(limit: Bytes) -> Result<Self> {
        Pages::try_from(limit)?; // ensure limit isn't too large
        Ok(Self { limit })
    }
}

impl<M: ModuleMod> Middleware<M> for MemoryChecker {
    type FM<'a> = DefaultFunctionMiddleware where M: 'a;

    fn update_module(&self, module: &mut M) -> Result<(), TransformError> {
        let Bytes(table_bytes) = module.table_bytes();
        let Bytes(limit) = self.limit;
        if table_bytes > limit {
            return Err(TransformError::new(
                "Memory Checker",
                "tables exceed memory limit",
            ));
        }
        let limit = limit.saturating_sub(table_bytes);
        let limit = Pages::try_from(Bytes(limit)).unwrap(); // checked in new()
        module.limit_memory(limit)
    }

    fn instrument<'a>(&self, _: LocalFunctionIndex) -> Result<Self::FM<'a>, TransformError>
    where
        M: 'a,
    {
        Ok(DefaultFunctionMiddleware)
    }
}
