// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use crate::util::{self, add_global};

use loupe::{MemoryUsage, MemoryUsageTracker};
use parking_lot::Mutex;
use wasmer::{
    FunctionMiddleware, GlobalInit, Instance, LocalFunctionIndex, MiddlewareError,
    MiddlewareReaderState, ModuleMiddleware, Type,
};
use wasmer_types::{GlobalIndex, ModuleInfo};
use wasmparser::{Operator, Type as WpType, TypeOrFuncType};

use std::mem;

#[derive(Debug)]
pub struct DepthChecker {
    global: Mutex<Option<GlobalIndex>>,
    limit: u32,
}

impl DepthChecker {
    pub fn new(limit: u32) -> Self {
        let global = Mutex::new(None);
        Self { global, limit }
    }
}

impl MemoryUsage for DepthChecker {
    fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + mem::size_of::<Option<GlobalIndex>>()
    }
}

impl ModuleMiddleware for DepthChecker {
    fn transform_module_info(&self, module: &mut ModuleInfo) {
        let limit = GlobalInit::I32Const(self.limit as i32);
        let space = add_global(module, "stack_space_left", Type::I32, limit);

        // also record the initial stack size limit
        add_global(module, "stack_size_limit", Type::I32, limit);

        let global = &mut *self.global.lock();
        assert_eq!(*global, None, "meter already set");
        *global = Some(space);
    }

    fn generate_function_middleware<'a>(
        &self,
        _: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware<'a> + 'a> {
        let global = self.global.lock().expect("no global");
        Box::new(FunctionDepthChecker::new(global))
    }
}

#[derive(Debug)]
struct FunctionDepthChecker<'a> {
    /// Represets the amount of stack depth remaining
    space: GlobalIndex,
    code: Vec<Operator<'a>>,
    scopes: usize,
}

impl<'a> FunctionDepthChecker<'a> {
    fn new(space: GlobalIndex) -> Self {
        Self {
            space,
            code: vec![],
            scopes: 0,
        }
    }
}

impl<'a> FunctionMiddleware<'a> for FunctionDepthChecker<'a> {
    fn feed(
        &mut self,
        operator: Operator<'a>,
        state: &mut MiddlewareReaderState<'a>,
    ) -> Result<(), MiddlewareError> {
        let last = self.scopes == 0 && matches!(operator, Operator::End);
        self.code.push(operator);

        if !last {
            return Ok(());
        }

        let global_index = self.space.as_u32();
        let size = 1;

        use Operator::*;
        state.extend(&[
            // if space <= size => panic with status = 1
            GlobalGet { global_index },
            I32Const { value: size },
            I32LtU,
            If {
                ty: TypeOrFuncType::Type(WpType::EmptyBlockType),
            },
            I32Const { value: 0 },
            GlobalSet { global_index },
            Unreachable,
            End,
            // space -= size
            GlobalGet { global_index },
            I32Const { value: size },
            I32Sub,
            GlobalSet { global_index },
        ]);

        println!("HERERERER");
        let code = std::mem::replace(&mut self.code, vec![]);

        let reclaim = |state: &mut MiddlewareReaderState<'a>| {
            state.extend(&[
                // space += size
                GlobalGet { global_index },
                I32Const { value: size },
                I32Add,
                GlobalSet { global_index },
            ])
        };

        for op in code {
            let exit = matches!(op, Return);
            if exit {
                reclaim(state);
            }
            state.push_operator(op);
        }

        reclaim(state);
        Ok(())
    }
}

pub fn stack_space_remaining(instance: &Instance) -> u32 {
    util::get_global(instance, "polyglot_stack_space_left")
}

pub fn stack_size(instance: &Instance) -> u32 {
    let limit: u32 = util::get_global(instance, "polyglot_stack_size_limit");
    let space: u32 = util::get_global(instance, "polyglot_stack_space_left");
    return limit - space;
}

pub fn reset_stack(instance: &Instance) {
    let limit: u32 = util::get_global(instance, "polyglot_stack_size_limit");
    util::set_global(instance, "polyglot_stack_space_left", limit);
}

pub fn set_stack_limit(instance: &Instance, new_limit: u32) {
    let limit: u32 = util::get_global(instance, "polyglot_stack_size_limit");
    let space: u32 = util::get_global(instance, "polyglot_stack_space_left");

    // space += the difference in the limits
    let space = space.saturating_add(new_limit).saturating_sub(limit);

    util::set_global(instance, "polyglot_stack_size_limit", new_limit);
    util::set_global(instance, "polyglot_stack_space_left", space);
}
