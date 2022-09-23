// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use loupe::MemoryUsage;
use wasmer::{
    wasmparser::{Operator, Type},
    MiddlewareError, ModuleMiddleware,
};
use wasmer_types::{
    Bytes, ExportIndex, GlobalIndex, GlobalInit, GlobalType, LocalFunctionIndex, ModuleInfo,
    Mutability, Pages, Type as WpType,
};

use std::{fmt::Debug, marker::PhantomData};

mod depth;
mod memory;
mod meter;
mod start;

pub trait ModuleMod {
    fn move_start_function(&mut self, name: &str);
    fn table_bytes(&self) -> Bytes;
    fn limit_memory(&mut self, limit: Pages);
    fn add_global(&mut self, name: &str, ty: WpType, init: GlobalInit) -> GlobalIndex;
}

// when GAT's are stabalized, move 'a to instrument
pub trait Middleware<'a> {
    type M: FunctionMiddleware<'a> + Debug + 'a;

    fn update_module(&self, module: &mut dyn ModuleMod);
    fn instrument(&self, func_index: LocalFunctionIndex) -> Self::M;
}

pub trait FunctionMiddleware<'a> {
    /// Provide info on the function's locals. This is called before feed.
    fn locals_info(&mut self, _locals: &[Type]) {}

    /// Processes the given operator.
    fn feed<O>(&mut self, op: Operator<'a>, out: &mut O) -> Result<(), String>
    where
        O: Extend<Operator<'a>>;
}

#[derive(Debug, MemoryUsage)]
pub struct DefaultFunctionMiddleware;

impl<'a> FunctionMiddleware<'a> for DefaultFunctionMiddleware {
    fn feed<O>(&mut self, op: Operator<'a>, out: &mut O) -> Result<(), String>
    where
        O: Extend<Operator<'a>>,
    {
        out.extend(vec![op]);
        Ok(())
    }
}

impl ModuleMod for ModuleInfo {
    fn move_start_function(&mut self, name: &str) {
        self.exports.remove(name);

        if let Some(start) = self.start_function.take() {
            let export = ExportIndex::Function(start);
            self.exports.insert(name.to_owned(), export);
            self.function_names.insert(start, name.to_owned());
        }
    }

    fn table_bytes(&self) -> Bytes {
        let mut total: u32 = 0;
        for (_, table) in &self.tables {
            // We don't support `TableGrow`, so the minimum is the size a table will always be.
            // We also don't support the 128-bit extension, so we'll say a `type` is at most 8 bytes.
            total = total.saturating_add(table.minimum.saturating_mul(8));
        }
        Bytes(total as usize)
    }

    fn limit_memory(&mut self, limit: Pages) {
        for (_, memory) in &mut self.memories {
            let limit = memory.maximum.unwrap_or(limit);
            let pages = limit.min(limit);
            memory.maximum = Some(pages);
        }
    }

    fn add_global(&mut self, name: &str, ty: WpType, init: GlobalInit) -> GlobalIndex {
        let global_type = GlobalType::new(ty, Mutability::Var);
        let name = name.to_owned();
        let index = self.globals.push(global_type);
        self.exports.insert(name, ExportIndex::Global(index));
        self.global_initializers.push(init);
        index
    }
}

#[derive(Debug, MemoryUsage)]
pub struct WasmerMiddlewareWrapper<T>(T)
where
    T: Debug + Send + Sync + MemoryUsage + for<'a> Middleware<'a>;

impl<T> ModuleMiddleware for WasmerMiddlewareWrapper<T>
where
    T: Debug + Send + Sync + MemoryUsage + for<'a> Middleware<'a>,
{
    fn transform_module_info(&self, module: &mut ModuleInfo) {
        self.0.update_module(module);
    }

    fn generate_function_middleware<'a>(
        &self,
        local_function_index: LocalFunctionIndex,
    ) -> Box<dyn wasmer::FunctionMiddleware<'a> + 'a> {
        Box::new(WasmerFunctionMiddlewareWrapper(
            self.0.instrument(local_function_index),
            PhantomData,
        ))
    }
}

#[derive(Debug)]
pub struct WasmerFunctionMiddlewareWrapper<'a, T: 'a>(T, PhantomData<&'a T>)
where
    T: Debug + FunctionMiddleware<'a>;

impl<'a, T> wasmer::FunctionMiddleware<'a> for WasmerFunctionMiddlewareWrapper<'a, T>
where
    T: Debug + FunctionMiddleware<'a>,
{
    fn locals_info(&mut self, locals: &[Type]) {
        self.0.locals_info(locals)
    }

    fn feed(
        &mut self,
        op: Operator<'a>,
        out: &mut wasmer::MiddlewareReaderState<'a>,
    ) -> Result<(), wasmer::MiddlewareError> {
        self.0
            .feed(op, out)
            .map_err(|err| MiddlewareError::new("Middleware", err))
    }
}
