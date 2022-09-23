// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use super::{DefaultFunctionMiddleware, Middleware, ModuleMod};

use wasmer_types::LocalFunctionIndex;

pub struct StartMover {
    name: String,
}

impl StartMover {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl<'a> Middleware<'a> for StartMover {
    type M = DefaultFunctionMiddleware;

    fn update_module(&self, module: &mut dyn ModuleMod) {
        module.move_start_function(&self.name);
    }

    fn instrument(&self, _: LocalFunctionIndex) -> Self::M {
        DefaultFunctionMiddleware
    }
}
