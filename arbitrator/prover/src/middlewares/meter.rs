// Copyright 2021-2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use super::{FunctionMiddleware, Middleware, ModuleMod};

use loupe::{MemoryUsage, MemoryUsageTracker};
use parking_lot::Mutex;
use wasmer::wasmparser::{Operator, Type as WpType, TypeOrFuncType};
use wasmer_types::{GlobalIndex, GlobalInit, LocalFunctionIndex, Type};

use std::{fmt::Debug, mem, sync::Arc};

pub struct Meter<F: Fn(&Operator) -> u64 + Send + Sync> {
    gas_global: Mutex<Option<GlobalIndex>>,
    status_global: Mutex<Option<GlobalIndex>>,
    costs: Arc<F>,
    start_gas: u64,
}

impl<F: Fn(&Operator) -> u64 + Send + Sync> Debug for Meter<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Meter")
            .field("gas_global", &self.gas_global)
            .field("status_global", &self.status_global)
            .field("costs", &"<function>")
            .field("start_gas", &self.start_gas)
            .finish()
    }
}

impl<F: Fn(&Operator) -> u64 + Send + Sync> MemoryUsage for Meter<F> {
    fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + 2 * mem::size_of::<Option<GlobalIndex>>() + mem::size_of::<u64>()
    }
}

impl<F: Fn(&Operator) -> u64 + Send + Sync> Meter<F> {
    pub fn new(costs: F, start_gas: u64) -> Self {
        Self {
            gas_global: Mutex::new(None),
            status_global: Mutex::new(None),
            costs: Arc::new(costs),
            start_gas,
        }
    }
}

impl<'a, F: Fn(&Operator) -> u64 + Send + Sync + 'static> Middleware<'a> for Meter<F> {
    type M = FunctionMeter<'a, F>;

    fn update_module(&self, module: &mut dyn ModuleMod) {
        let start = GlobalInit::I64Const(self.start_gas as i64);
        let gas = module.add_global("gas_left", Type::I64, start);
        let status = module.add_global("gas_status", Type::I32, GlobalInit::I32Const(0));
        *self.gas_global.lock() = Some(gas);
        *self.status_global.lock() = Some(status);
    }

    fn instrument(&self, _: LocalFunctionIndex) -> Self::M {
        let gas = self.gas_global.lock().expect("no global");
        let status = self.gas_global.lock().expect("no global");
        FunctionMeter::new(gas, status, self.costs.clone())
    }
}

pub struct FunctionMeter<'a, F: Fn(&Operator) -> u64 + Send + Sync> {
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

impl<'a, F: Fn(&Operator) -> u64 + Send + Sync> FunctionMiddleware<'a> for FunctionMeter<'a, F> {
    fn feed<O>(&mut self, op: Operator<'a>, out: &mut O) -> Result<(), String>
    where
        O: Extend<Operator<'a>>,
    {
        use Operator::*;

        macro_rules! dot {
            ($first:ident $(,$opcode:ident)*) => {
                $first { .. } $(| $opcode { .. })*
            };
        }

        let end = matches!(
            op,
            End | Else | Return | dot!(Loop, Br, BrTable, BrIf, Call, CallIndirect)
        );

        let cost = self.block_cost.saturating_add((self.costs)(&op));
        self.block_cost = cost;
        self.block.push(op);

        if end {
            let gas = self.gas_global.as_u32();
            let status = self.status_global.as_u32();

            out.extend(vec![
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

            out.extend(self.block.clone());
            self.block.clear();
            self.block_cost = 0;
        }
        Ok(())
    }
}
