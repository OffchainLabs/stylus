// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use super::{DefaultFunctionMiddleware, Middleware, ModuleMod, TransformError};

use loupe::MemoryUsage;
use wasmer_types::LocalFunctionIndex;

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

impl<M: ModuleMod> Middleware<M> for StartMover {
    type FM<'a> = DefaultFunctionMiddleware where M: 'a;

    fn update_module(&self, module: &mut M) -> Result<(), TransformError> {
        module.move_start_function(&self.name);
        Ok(())
    }

    fn instrument<'a>(&self, _: LocalFunctionIndex) -> Result<Self::FM<'a>, TransformError>
    where
        M: 'a,
    {
        Ok(DefaultFunctionMiddleware)
    }
}
