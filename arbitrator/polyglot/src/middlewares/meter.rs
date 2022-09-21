// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use crate::middlewares::{add_global, get_global, set_global};

use loupe::{MemoryUsage, MemoryUsageTracker};
use parking_lot::Mutex;
use wasmer::{
    FunctionMiddleware, GlobalInit, Instance, LocalFunctionIndex, MiddlewareError,
    MiddlewareReaderState, ModuleMiddleware, Type,
};
use wasmer_types::{GlobalIndex, ModuleInfo};
use wasmparser::{Operator, Type as WpType, TypeOrFuncType};

use std::{fmt::Debug, mem, sync::Arc};

pub struct Meter<F: Fn(&Operator) -> u64 + Send + Sync> {
    globals: Mutex<Option<(GlobalIndex, GlobalIndex)>>,
    costs: Arc<F>,
    start_gas: u64,
}

impl<F: Fn(&Operator) -> u64 + Send + Sync> Meter<F> {
    pub fn new(costs: F, start_gas: u64) -> Self {
        Self {
            globals: Mutex::new(None),
            costs: Arc::new(costs),
            start_gas,
        }
    }
}

impl<F: Fn(&Operator) -> u64 + Send + Sync> Debug for Meter<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Meter")
            .field("globals", &self.globals)
            .field("costs", &"<function>")
            .finish()
    }
}

impl<F: Fn(&Operator) -> u64 + Send + Sync> MemoryUsage for Meter<F> {
    fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + mem::size_of::<Option<(GlobalIndex, GlobalIndex)>>()
    }
}

impl<F: Fn(&Operator) -> u64 + Send + Sync + 'static> ModuleMiddleware for Meter<F> {
    fn transform_module_info(&self, module: &mut ModuleInfo) {
        let start = GlobalInit::I64Const(self.start_gas as i64);
        let gas = add_global(module, "gas_left", Type::I64, start);
        let status = add_global(module, "gas_status", Type::I32, GlobalInit::I32Const(0));
        let global = &mut *self.globals.lock();
        assert_eq!(*global, None, "meter already set");
        *global = Some((gas, status));
    }

    fn generate_function_middleware<'a>(
        &self,
        _: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware<'a> + 'a> {
        let (gas, status) = self.globals.lock().expect("no globals");
        Box::new(FunctionMeter::new(gas, status, self.costs.clone()))
    }
}

struct FunctionMeter<'a, F: Fn(&Operator) -> u64 + Send + Sync> {
    /// Represents the amount of gas left for consumption
    gas_global: GlobalIndex,
    /// Represents whether the machine is out of gas
    status_global: GlobalIndex,
    /// Instructions of the current basic block
    block: Vec<Operator<'a>>,
    /// The accumulated cost of the current basic block
    block_cost: u64,
    /// Associates opcodes to their gas costs
    costs: Arc<F>,
}

impl<'a, F: Fn(&Operator) -> u64 + Send + Sync> FunctionMeter<'a, F> {
    fn new(gas_global: GlobalIndex, status_global: GlobalIndex, costs: Arc<F>) -> Self {
        Self {
            gas_global,
            status_global,
            block: vec![],
            block_cost: 0,
            costs,
        }
    }
}

impl<F: Fn(&Operator) -> u64 + Send + Sync> Debug for FunctionMeter<'_, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionMeter")
            .field("gas_global", &self.gas_global)
            .field("status_global", &self.status_global)
            .field("block", &self.block)
            .field("block_cost", &self.block_cost)
            .field("costs", &"<function>")
            .finish()
    }
}

impl<'a, F: Fn(&Operator) -> u64 + Send + Sync> FunctionMiddleware<'a> for FunctionMeter<'a, F> {
    fn feed(
        &mut self,
        operator: Operator<'a>,
        state: &mut MiddlewareReaderState<'a>,
    ) -> Result<(), MiddlewareError> {
        use Operator::*;

        macro_rules! dot {
            ($first:ident $(,$opcode:ident)*) => {
                $first { .. } $(| $opcode { .. })*
            };
        }

        let end = matches!(
            operator,
            End | Else | Return | dot!(Loop, Br, BrTable, BrIf, Call, CallIndirect)
        );

        let cost = self.block_cost.saturating_add((self.costs)(&operator));
        self.block_cost = cost;
        self.block.push(operator);

        if end {
            let gas = self.gas_global.as_u32();
            let status = self.status_global.as_u32();

            state.extend(&[
                // if gas < cost => panic with status = 1
                GlobalGet { global_index: gas },
                I64Const { value: cost as i64 },
                I64LtU,
                If {
                    ty: TypeOrFuncType::Type(WpType::EmptyBlockType),
                },
                I32Const { value: 1 },
                GlobalSet {
                    global_index: status,
                },
                Unreachable,
                End,
                // gas -= cost
                GlobalGet { global_index: gas },
                I64Const { value: cost as i64 },
                I64Sub,
                GlobalSet { global_index: gas },
            ]);

            state.extend(&self.block);
            self.block.clear();
            self.block_cost = 0;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum MachineMeter {
    Ready(u64),
    Exhausted,
}

pub fn gas_left(instance: &Instance) -> MachineMeter {
    let gas = get_global(instance, "polyglot_gas_left");
    let status: i32 = get_global(instance, "polyglot_gas_status");

    return match status == 1 {
        true => MachineMeter::Exhausted,
        false => MachineMeter::Ready(gas),
    };
}

pub fn set_gas(instance: &Instance, gas: u64) {
    set_global(instance, "polyglot_gas_left", gas);
    set_global(instance, "polyglot_gas_status", 0);
}
