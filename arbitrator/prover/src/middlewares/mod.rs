// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use crate::{
    binary::WasmBinary,
    value::{FunctionType as ArbFunctionType, Value},
};
use loupe::MemoryUsage;
use wasmer::{
    wasmparser::{Operator, Type},
    MiddlewareError, ModuleMiddleware,
};
use wasmer_types::{
    Bytes, ExportIndex, FunctionIndex, GlobalIndex, GlobalInit, GlobalType,
    LocalFunctionIndex, ModuleInfo, Mutability, Pages, SignatureIndex, Type as WpType,
};
use std::{
    convert::TryInto,
    fmt::Debug,
    marker::{PhantomData, Send, Sync},
};

pub mod depth;
pub mod memory;
pub mod meter;
pub mod start;

pub trait ModuleMod: Clone + Debug + Send + Sync {
    fn move_start_function(&mut self, name: &str);
    fn table_bytes(&self) -> Bytes;
    fn limit_memory(&mut self, limit: Pages);
    fn add_global(&mut self, name: &str, ty: WpType, init: GlobalInit) -> GlobalIndex;
    fn get_signature(&self, sig: SignatureIndex) -> Result<ArbFunctionType, String>;
    fn get_function(&self, func: FunctionIndex) -> Result<ArbFunctionType, String>;
}

// when GAT's are stabalized, move 'a to instrument
pub trait Middleware<'a, M: ModuleMod> {
    type FM: FunctionMiddleware<'a> + Debug + 'a;

    fn update_module(&self, module: &mut M); // not mutable due to wasmer
    fn instrument(&self, func_index: LocalFunctionIndex) -> Self::FM;
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

    fn get_signature(&self, sig: SignatureIndex) -> Result<ArbFunctionType, String> {
        let index = sig.as_u32();
        let error = || format!("missing signature {index}");
        let ty = self.signatures.get(sig).ok_or_else(error)?;
        ty.clone().try_into().map_err(|_| error())
    }

    fn get_function(&self, func: FunctionIndex) -> Result<ArbFunctionType, String> {
        match self.functions.get(func) {
            Some(sig) => self.get_signature(*sig),
            None => return Err(format!("missing func with id {}", func.as_u32())),
        }
    }
}

impl<'a> ModuleMod for WasmBinary<'a> {
    fn move_start_function(&mut self, name: &str) {
        self.exports.remove(name);

        if let Some(start) = self.start.take() {
            self.exports.insert(name.to_owned(), start);
            self.names.functions.insert(start, name.to_owned());
        }
    }

    fn table_bytes(&self) -> Bytes {
        let mut total: u32 = 0;
        for table in &self.tables {
            // We don't support `TableGrow`, so the minimum is the size a table will always be.
            // We also don't support the 128-bit extension, so we'll say a `type` is at most 8 bytes.
            total = total.saturating_add(table.initial.saturating_mul(8));
        }
        Bytes(total as usize)
    }

    fn limit_memory(&mut self, limit: Pages) {
        for memory in &mut self.memories {
            let Pages(limit) = limit;
            let limit = memory.maximum.unwrap_or(limit.into());
            let pages = limit.min(limit);
            memory.maximum = Some(pages);
        }
    }

    fn add_global(&mut self, name: &str, _ty: WpType, init: GlobalInit) -> GlobalIndex {
        let global = match init {
            GlobalInit::I32Const(x) => Value::I32(x as u32),
            GlobalInit::I64Const(x) => Value::I64(x as u64),
            GlobalInit::F32Const(x) => Value::F32(x),
            GlobalInit::F64Const(x) => Value::F64(x),
            x => panic!("cannot add global of type {:?}", x),
        };

        let index = GlobalIndex::from_u32(self.globals.len() as u32);
        self.globals.push(global);
        self.exports.insert(name.to_owned(), index.as_u32());
        index
    }

    fn get_signature(&self, sig: SignatureIndex) -> Result<ArbFunctionType, String> {
        let index = sig.as_u32() as usize;
        let error = || format!("missing signature {index}");
        let ty = self.types.get(index).ok_or_else(error)?;
        ty.clone().try_into().map_err(|_| error())
    }

    fn get_function(&self, func: FunctionIndex) -> Result<ArbFunctionType, String> {
        let index = func.as_u32() as usize;
        match self.functions.get(index) {
            Some(sig) => self.get_signature(SignatureIndex::from_u32(*sig)),
            None => return Err(format!("missing func with id {}", func.as_u32())),
        }
    }
}

#[derive(Debug, MemoryUsage)]
pub struct WasmerMiddlewareWrapper<T, M>(pub T, PhantomData<M>)
where
    M: ModuleMod,
    T: Debug + Send + Sync + MemoryUsage + for<'a> Middleware<'a, M>;

impl<T, M> WasmerMiddlewareWrapper<T, M>
where
    M: ModuleMod,
    T: Debug + Send + Sync + MemoryUsage + for<'a> Middleware<'a, M>,
{
    pub fn new(middleware: T) -> Self {
        WasmerMiddlewareWrapper(middleware, PhantomData)
    }
}

impl<T> ModuleMiddleware for WasmerMiddlewareWrapper<T, ModuleInfo>
where
    T: Debug + Send + Sync + MemoryUsage + for<'a> Middleware<'a, ModuleInfo>,
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
