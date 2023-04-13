// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use super::{Middleware, ModuleMod, FuncMiddleware};
use arbutil::DebugColor;
use eyre::Result;
use wasmer_types::LocalFunctionIndex;
use wasmparser::Operator;

#[derive(Debug, Default)]
pub struct Printer {}

impl<M: ModuleMod> Middleware<M> for Printer {
    type FM<'a> = FuncPrinter;

    fn update_module(&self, _module: &mut M) -> Result<()> {
        Ok(())
    }

    fn instrument<'a>(&self, func: LocalFunctionIndex) -> Result<Self::FM<'a>> {
        Ok(FuncPrinter::new(func))
    }

    fn name(&self) -> &'static str {
        "printer"
    }
}

#[derive(Debug)]
pub struct FuncPrinter {
    func: LocalFunctionIndex,
}

impl FuncPrinter {
    fn new(func: LocalFunctionIndex) -> Self {
        Self { func }
    }
}

impl<'a> FuncMiddleware<'a> for FuncPrinter {
    fn feed<O>(&mut self, op: Operator<'a>, out: &mut O) -> Result<()>
    where
        O: Extend<Operator<'a>>,
    {
        println!("{} {}", self.func.as_u32(), op.debug_grey());
        out.extend([op]);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "printer"
    }
}
