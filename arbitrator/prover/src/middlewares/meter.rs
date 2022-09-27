// Copyright 2021-2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use crate::{Machine, Value};

use super::{FunctionMiddleware, GlobalMod, Middleware, ModuleMod};

use loupe::{MemoryUsage, MemoryUsageTracker};
use parking_lot::Mutex;
use wasmer::wasmparser::{Operator, Type as WpType, TypeOrFuncType};
use wasmer::{Instance, MiddlewareError};
use wasmer_types::{GlobalIndex, GlobalInit, LocalFunctionIndex, Type};

use std::fmt::Display;
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

impl<'a, M, F> Middleware<'a, M> for Meter<F>
where
    M: ModuleMod,
    F: Fn(&Operator) -> u64 + Send + Sync + 'static,
{
    type FM = FunctionMeter<'a, F>;

    fn update_module(&self, module: &mut M) -> Result<(), MiddlewareError> {
        let start = GlobalInit::I64Const(self.start_gas as i64);
        let gas = module.add_global("polyglot_gas_left", Type::I64, start);
        let status = module.add_global("polyglot_gas_status", Type::I32, GlobalInit::I32Const(0));
        *self.gas_global.lock() = Some(gas);
        *self.status_global.lock() = Some(status);
        Ok(())
    }

    fn instrument(&self, _: LocalFunctionIndex) -> Result<Self::FM, MiddlewareError> {
        let gas = self.gas_global.lock().expect("no global");
        let status = self.status_global.lock().expect("no global");
        Ok(FunctionMeter::new(gas, status, self.costs.clone()))
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

        let mut cost = self.block_cost.saturating_add((self.costs)(&op));
        self.block_cost = cost;
        self.block.push(op);

        if end {
            let gas = self.gas_global.as_u32();
            let status = self.status_global.as_u32();

            let mut header = vec![
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
            ];

            // include the cost of executing the header
            for op in &header {
                cost = cost.saturating_add((self.costs)(op))
            }
            header[1] = I64Const { value: cost as i64 };
            header[9] = I64Const { value: cost as i64 };
            
            out.extend(header);
            out.extend(self.block.clone());
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

impl Display for MachineMeter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready(gas) => write!(f, "{} gas left", gas),
            Self::Exhausted => write!(f, "out of gas"),
        }
    }
}

impl Into<u64> for MachineMeter {
    fn into(self) -> u64 {
        match self {
            Self::Ready(gas) => gas,
            Self::Exhausted => 0,
        }
    }
}

pub trait MeteredMachine {
    fn gas_left(&self) -> MachineMeter;
    fn set_gas(&mut self, gas: u64);
}

const METER_ERROR: &str = "machine not instrumented with metering code";
const TYPE_ERROR: &str = "wrong type for metering instrumentation";

impl MeteredMachine for Machine {
    fn gas_left(&self) -> MachineMeter {
        let gas = self.get_global("polyglot_gas_left").expect(METER_ERROR);
        let status = self.get_global("polyglot_gas_status").expect(METER_ERROR);
        let gas = match gas {
            Value::I64(gas) => gas,
            _ => panic!("{}", TYPE_ERROR),
        };
        match status {
            Value::I32(1) => MachineMeter::Exhausted,
            Value::I32(0) => MachineMeter::Ready(gas),
            _ => panic!("{}", TYPE_ERROR),
        }
    }

    fn set_gas(&mut self, gas: u64) {
        self.set_global("polyglot_gas_left", Value::I64(gas))
            .expect(METER_ERROR);
        self.set_global("polyglot_gas_status", Value::I32(0))
            .expect(METER_ERROR);
    }
}

impl MeteredMachine for Instance {
    fn gas_left(&self) -> MachineMeter {
        let gas = self.get_global("polyglot_gas_left");
        let status: i32 = self.get_global("polyglot_gas_status");

        match status == 1 {
            true => MachineMeter::Exhausted,
            false => MachineMeter::Ready(gas),
        }
    }

    fn set_gas(&mut self, gas: u64) {
        self.set_global("polyglot_gas_left", gas);
        self.set_global("polyglot_gas_status", 0);
    }
}
