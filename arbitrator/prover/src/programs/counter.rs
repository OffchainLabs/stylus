// Copyright 2021-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use super::{FuncMiddleware, Middleware, ModuleMod};
use crate::Machine;

use arbutil::operator::{OperatorCode, OperatorInfo};
use eyre::{eyre, Result};
use fnv::FnvHashMap as HashMap;
use parking_lot::Mutex;
use wasmer::LocalFunctionIndex;
use std::collections::BTreeMap;
use std::{fmt::Debug, sync::Arc};
use wasmer_types::{GlobalIndex, GlobalInit, Type};
use wasmparser::Operator;

#[derive(Debug)]
pub struct Counter {
    /// Assigns each relative offset a global variable
    pub counters: Arc<Mutex<Vec<GlobalIndex>>>,
}

impl Counter {
    pub fn new() -> Self {
        let counters = Arc::new(Mutex::new(Vec::with_capacity(OperatorCode::COUNT)));
        Self { counters }
    }

    pub fn global_name(index: usize) -> String {
        format!("stylus_opcode{}_count", index)
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Middleware<M> for Counter
where
    M: ModuleMod,
{
    type FM<'a> = FuncCounter<'a>;

    fn update_module(&self, module: &mut M) -> Result<()> {
        let mut counters = self.counters.lock();
        for index in 0..OperatorCode::COUNT {
            let zero_count = GlobalInit::I64Const(0);
            let global = module.add_global(&Self::global_name(index), Type::I64, zero_count)?;
            counters.push(global);
        }
        Ok(())
    }

    fn instrument<'a>(&self, _: LocalFunctionIndex) -> Result<Self::FM<'a>> {
        Ok(FuncCounter::new(self.counters.clone()))
    }

    fn name(&self) -> &'static str {
        "operator counter"
    }
}

#[derive(Debug)]
pub struct FuncCounter<'a> {
    /// Assigns each relative offset a global variable
    counters: Arc<Mutex<Vec<GlobalIndex>>>,
    /// Instructions of the current basic block
    block: Vec<Operator<'a>>,
}

impl<'a> FuncCounter<'a> {
    fn new(counters: Arc<Mutex<Vec<GlobalIndex>>>) -> Self {
        let block = vec![];
        Self { counters, block }
    }
}

impl<'a> FuncMiddleware<'a> for FuncCounter<'a> {
    fn feed<O>(&mut self, op: Operator<'a>, out: &mut O) -> Result<()>
    where
        O: Extend<Operator<'a>>,
    {
        use Operator::*;

        let end = op.ends_basic_block();
        self.block.push(op);

        if end {
            let update = |global_index: u32, value: i64| {
                [
                    GlobalGet { global_index },
                    I64Const { value },
                    I64Add,
                    GlobalSet { global_index },
                ]
            };

            // there's always at least one op, so we chain the instrumentation
            let mut increments = HashMap::default();
            for op in self.block.iter().chain(update(0, 0).iter()) {
                let count = increments.entry(op.code()).or_default();
                *count += 1;
            }

            // add the instrumentation's contribution to the overall counts
            let kinds = increments.len() as i64;
            for op in update(0, 0) {
                let count = increments.get_mut(&op.code()).unwrap();
                *count += kinds - 1; // we included one in the last loop
            }

            let counters = self.counters.lock();
            for (op, count) in increments {
                let global = *counters.get(op.seq()).ok_or_else(|| eyre!("no global"))?;
                out.extend(update(global.as_u32(), count));
            }

            out.extend(self.block.drain(..));
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "operator counter"
    }
}

pub trait CountingMachine {
    fn operator_count(&mut self, op: OperatorCode) -> Result<usize>;

    fn operator_counts(&mut self) -> Result<BTreeMap<OperatorCode, usize>> {
        let mut counts = BTreeMap::new();
        for op in OperatorCode::op_iter() {
            let count = self.operator_count(op)?;
            if count != 0 {
                counts.insert(op, count as usize);
            }
        }
        Ok(counts)
    }
}

impl CountingMachine for Machine {
    fn operator_count(&mut self, op: OperatorCode) -> Result<usize> {
        let count = self.get_global(&Counter::global_name(op.seq()))?;
        let count: u64 = count.try_into()?;
        Ok(count as usize)
    }
}
