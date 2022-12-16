// Copyright 2021-2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use crate::{Machine, Value};

use super::{FunctionMiddleware, Middleware, ModuleMod, TransformError};

use loupe::{MemoryUsage, MemoryUsageTracker};
use parking_lot::Mutex;
use wasmer_types::{GlobalIndex, GlobalInit, LocalFunctionIndex, Type};
use wasmparser::{Operator};

use std::collections::HashMap;
use std::convert::TryInto;

use std::{clone::Clone, fmt::Debug, mem, sync::Arc};

#[cfg(feature = "native")]
use {super::GlobalMod, wasmer::Instance};

const MAX_UNIQUE_OPCODE_COUNT: usize = 256;

macro_rules! opcode_count_name {
    ($val:expr) => {
        format!("polyglot_opcode{}_count", $val)
    }
}

#[derive(Debug)]
pub struct Counter {
    pub index_counts_global: Arc<Mutex<Vec<GlobalIndex>>>,
    pub opcode_indexes: Arc<Mutex<HashMap<usize, usize>>>,
}

impl MemoryUsage for Counter {
    fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + mem::size_of::<GlobalIndex>() * MAX_UNIQUE_OPCODE_COUNT
            + (mem::size_of::<usize>() * 2) * MAX_UNIQUE_OPCODE_COUNT
    }
}

impl Counter {
    pub fn new(index_counts_global: Arc<Mutex<Vec<GlobalIndex>>>, opcode_indexes: Arc<Mutex<HashMap<usize, usize>>>) -> Self {
        Self {
            index_counts_global,
            opcode_indexes
        }
    }
}

impl<M> Middleware<M> for Counter
where
    M: ModuleMod,
{
    type FM<'a> = FunctionCounter<'a> where M: 'a;

    fn update_module(&self, module: &mut M) -> Result<(), TransformError> {
        let zero_count = GlobalInit::I64Const(0);
        for (index, global_index) in self.index_counts_global.lock().iter_mut().enumerate() {
            *global_index = module.add_global(opcode_count_name!(index).as_str(), Type::I64, zero_count);
        }
        Ok(())
    }

    fn instrument<'a>(&self, _: LocalFunctionIndex) -> Result<Self::FM<'a>, TransformError>
    where
        M: 'a,
    {
        Ok(FunctionCounter::new(self.index_counts_global.clone(), self.opcode_indexes.clone()))
    }
}

#[derive(Debug)]
pub struct FunctionCounter<'a> {
    /// WASM global variables to keep track of opcode counts
    index_counts_global: Arc<Mutex<Vec<GlobalIndex>>>,
    ///  Mapping of operator code to index for opcode_counts_global and block_opcode_counts
    opcode_indexes: Arc<Mutex<HashMap<usize, usize>>>,
    /// Instructions of the current basic block
    block: Vec<Operator<'a>>,
    /// Number of times each opcode was used in current basic block
    block_index_counts: Vec<u64>,
}

impl<'a> FunctionCounter<'a> {
    fn new(index_counts_global: Arc<Mutex<Vec<GlobalIndex>>>, opcode_indexes: Arc<Mutex<HashMap<usize, usize>>>) -> Self {
        let max = index_counts_global.lock().len();
        Self {
            index_counts_global,
            opcode_indexes,
            block: vec![],
            block_index_counts: vec![0; max],
        }
    }
}

impl<'a> FunctionMiddleware<'a> for FunctionCounter<'a> {
    fn feed<O>(&mut self, op: Operator<'a>, out: &mut O) -> Result<(), String>
    where
        O: Extend<Operator<'a>>,
    {
        use arbutil::operator::operator_lookup_code;
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

        let code = operator_lookup_code(&op);
        let mut opcode_indexes = self.opcode_indexes.lock();
        let next = opcode_indexes.len();
        let index = opcode_indexes.entry(code).or_insert(next);
        assert!(*index > MAX_UNIQUE_OPCODE_COUNT, "too many unique opcodes {next}");
        self.block_index_counts[*index] += 1;
        self.block.push(op);

        if end {
            let index_counts_global = self.index_counts_global.lock();
            for (index, count) in self.block_index_counts.iter().enumerate() {
                if *count > 0 {
                    let global_index = index_counts_global[index].as_u32();
                    let add_single_count = vec![
                        GlobalGet { global_index },
                        I64Const { value: *count as i64 },
                        I64Add,
                        GlobalSet { global_index },
                    ];
                    out.extend(add_single_count);
                }
            }

            out.extend(self.block.clone());
            self.block.clear();
            self.block_index_counts = vec![0; index_counts_global.len()]
        }
        Ok(())
    }
}

pub trait CountedMachine {
    fn opcode_counts(&self) -> Vec<u64>;
    fn set_opcode_counts(&mut self, index_counts: Vec<u64>);
}

const COUNTER_ERROR: &str = "machine not instrumented with opcode counting code";
const TYPE_ERROR: &str = "wrong type for opcode counting instrumentation";

impl CountedMachine for Machine {
    fn opcode_counts(&self) -> Vec<u64> {
        let mut counts = Vec::new();
        for i in 0..MAX_UNIQUE_OPCODE_COUNT {
            let count = self.get_global(opcode_count_name!(i).as_str()).expect(COUNTER_ERROR);
            let count: u64 = count.try_into().expect(TYPE_ERROR);
            if count == 0 {
                break;
            }

            counts.push(count)
        }

        counts
    }

    fn set_opcode_counts(&mut self, index_counts: Vec<u64>) {
        for (index, count) in index_counts.iter().enumerate() {
            self.set_global(opcode_count_name!(index).as_str(), Value::I64(*count)).expect(COUNTER_ERROR);
        }
    }
}

#[cfg(feature = "native")]
impl CountedMachine for Instance {
    fn opcode_counts(&self) -> Vec<u64> {
        let mut counts = Vec::new();
        for i in 0..MAX_UNIQUE_OPCODE_COUNT {
            let count = self.get_global(opcode_count_name!(i).as_str());

            if count == 0 {
                break;
            }

            counts.push(count)
        }

        counts
    }

    fn set_opcode_counts(&mut self, index_counts: Vec<u64>) {
        for (index, count) in index_counts.iter().enumerate() {
            self.set_global(opcode_count_name!(index).as_str(), *count);
        }
    }
}
