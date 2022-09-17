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
    /// The maximum size of the stack, measured in words
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

fn worst_case_depth<'a>(code: &[Operator<'a>]) -> Result<u32, MiddlewareError> {
    use Operator::*;

    let mut stack: u32 = 0;

    macro_rules! push {
        ($count:expr) => {{
            stack += $count;
        }};
        () => {
            push!(1)
        };
    }
    macro_rules! pop {
        ($count:expr) => {{
            stack = stack.saturating_sub($count);
        }};
        () => {
            pop!(1)
        };
    }
    macro_rules! op {
        ($first:ident $(,$opcode:ident)* $(,)?) => {
            $first $(| $opcode)*
        };
    }
    macro_rules! dot {
        ($first:ident $(,$opcode:ident)* $(,)?) => {
            $first { .. } $(| $opcode { .. })*
        };
    }
    macro_rules! error {
        ($text:expr $(,$args:expr)*) => {{
            let name = "depth-checker failure".to_owned();
            let message = format!($text $(,$args)*);
            return Err(MiddlewareError{
                name,
                message,
            });
        }}
    }

    for op in code {
        #[rustfmt::skip]
        match op {

            op!(
                Nop,
                I32Eqz, I64Eqz, I32Clz, I32Ctz, I32Popcnt, I64Clz, I64Ctz, I64Popcnt,
            )
            | dot!(
                LocalTee, MemoryGrow,
                I32Load, I64Load, F32Load, F64Load,
                I32Load8S, I32Load8U, I32Load16S, I32Load16U, I64Load8S, I64Load8U,
                I64Load16S, I64Load16U, I64Load32S, I64Load32U,
                I32WrapI64, I64ExtendI32S, I64ExtendI32U,
                I32Extend8S, I32Extend16S, I64Extend8S, I64Extend16S, I64Extend32S
            ) => {}

            dot!(
                LocalGet, GlobalGet, MemorySize,
                I32Const, I64Const, F32Const, F64Const,
            ) => push!(),

            op!(
                Drop,
                I32Eq, I32Ne, I32LtS, I32LtU, I32GtS, I32GtU, I32LeS, I32LeU, I32GeS, I32GeU,
                I64Eq, I64Ne, I64LtS, I64LtU, I64GtS, I64GtU, I64LeS, I64LeU, I64GeS, I64GeU,
                F32Eq, F32Ne, F32Lt, F32Gt, F32Le, F32Ge,
                F64Eq, F64Ne, F64Lt, F64Gt, F64Le, F64Ge,
                I32Add, I32Sub, I32Mul, I32DivS, I32DivU, I32RemS, I32RemU,
                I64Add, I64Sub, I64Mul, I64DivS, I64DivU, I64RemS, I64RemU,
                I32And, I32Or, I32Xor, I32Shl, I32ShrS, I32ShrU, I32Rotl, I32Rotr,
                I64And, I64Or, I64Xor, I64Shl, I64ShrS, I64ShrU, I64Rotl, I64Rotr,
            )
            | dot!(LocalSet, GlobalSet) => pop!(),

            dot!(
                Select,
                I32Store, I64Store, F32Store, F64Store, I32Store8, I32Store16, I64Store8, I64Store16, I64Store32,
            ) => pop!(2),

            unsupported @ dot!(Try, Catch, Throw, Rethrow) => {
                error!("exception-handling extension not supported {:?}", unsupported)
            },

            unsupported @ dot!(TypedSelect) => {
                error!("reference-types extension not supported {:?}", unsupported)
            },

            unsupported @ (
                dot!(
                    MemoryInit, DataDrop, MemoryCopy, MemoryFill, TableInit, ElemDrop,
                    TableCopy, TableFill, TableGet, TableSet, TableGrow, TableSize
                )
            ) => error!("bulk-memory-operations extension not supported {:?}", unsupported),

            unsupported @ (
                dot!(
                    MemoryAtomicNotify, MemoryAtomicWait32, MemoryAtomicWait64, AtomicFence, I32AtomicLoad,
                    I64AtomicLoad, I32AtomicLoad8U, I32AtomicLoad16U, I64AtomicLoad8U, I64AtomicLoad16U,
                    I64AtomicLoad32U, I32AtomicStore, I64AtomicStore, I32AtomicStore8, I32AtomicStore16,
                    I64AtomicStore8, I64AtomicStore16, I64AtomicStore32, I32AtomicRmwAdd, I64AtomicRmwAdd,
                    I32AtomicRmw8AddU, I32AtomicRmw16AddU, I64AtomicRmw8AddU, I64AtomicRmw16AddU, I64AtomicRmw32AddU,
                    I32AtomicRmwSub, I64AtomicRmwSub, I32AtomicRmw8SubU, I32AtomicRmw16SubU, I64AtomicRmw8SubU,
                    I64AtomicRmw16SubU, I64AtomicRmw32SubU, I32AtomicRmwAnd, I64AtomicRmwAnd, I32AtomicRmw8AndU,
                    I32AtomicRmw16AndU, I64AtomicRmw8AndU, I64AtomicRmw16AndU, I64AtomicRmw32AndU, I32AtomicRmwOr,
                    I64AtomicRmwOr, I32AtomicRmw8OrU, I32AtomicRmw16OrU, I64AtomicRmw8OrU, I64AtomicRmw16OrU,
                    I64AtomicRmw32OrU, I32AtomicRmwXor, I64AtomicRmwXor, I32AtomicRmw8XorU, I32AtomicRmw16XorU,
                    I64AtomicRmw8XorU, I64AtomicRmw16XorU, I64AtomicRmw32XorU, I32AtomicRmwXchg, I64AtomicRmwXchg,
                    I32AtomicRmw8XchgU, I32AtomicRmw16XchgU, I64AtomicRmw8XchgU, I64AtomicRmw16XchgU,
                    I64AtomicRmw32XchgU, I32AtomicRmwCmpxchg, I64AtomicRmwCmpxchg, I32AtomicRmw8CmpxchgU,
                    I32AtomicRmw16CmpxchgU, I64AtomicRmw8CmpxchgU, I64AtomicRmw16CmpxchgU, I64AtomicRmw32CmpxchgU
                )
            ) => error!("threads extension not supported {:?}", unsupported),

            unsupported @ (
                dot!(
                    V128Load, V128Load8x8S, V128Load8x8U, V128Load16x4S, V128Load16x4U, V128Load32x2S, V128Load32x2U,
                    V128Load8Splat, V128Load16Splat, V128Load32Splat, V128Load64Splat, V128Load32Zero, V128Load64Zero,
                    V128Store, V128Load8Lane, V128Load16Lane, V128Load32Lane, V128Load64Lane, V128Store8Lane,
                    V128Store16Lane, V128Store32Lane, V128Store64Lane, V128Const,
                    I8x16Shuffle, I8x16ExtractLaneS, I8x16ExtractLaneU, I8x16ReplaceLane, I16x8ExtractLaneS,
                    I16x8ExtractLaneU, I16x8ReplaceLane, I32x4ExtractLane, I32x4ReplaceLane, I64x2ExtractLane,
                    I64x2ReplaceLane, F32x4ExtractLane, F32x4ReplaceLane, F64x2ExtractLane, F64x2ReplaceLane,
                    I8x16Swizzle, I8x16Splat, I16x8Splat, I32x4Splat, I64x2Splat, F32x4Splat, F64x2Splat, I8x16Eq,
                    I8x16Ne, I8x16LtS, I8x16LtU, I8x16GtS, I8x16GtU, I8x16LeS, I8x16LeU, I8x16GeS, I8x16GeU, I16x8Eq,
                    I16x8Ne, I16x8LtS, I16x8LtU, I16x8GtS, I16x8GtU, I16x8LeS, I16x8LeU, I16x8GeS, I16x8GeU, I32x4Eq,
                    I32x4Ne, I32x4LtS, I32x4LtU, I32x4GtS, I32x4GtU, I32x4LeS, I32x4LeU, I32x4GeS, I32x4GeU, I64x2Eq,
                    I64x2Ne, I64x2LtS, I64x2GtS, I64x2LeS, I64x2GeS,
                    F32x4Eq, F32x4Ne, F32x4Lt, F32x4Gt, F32x4Le, F32x4Ge,
                    F64x2Eq, F64x2Ne, F64x2Lt, F64x2Gt, F64x2Le, F64x2Ge,
                    V128Not, V128And, V128AndNot, V128Or, V128Xor, V128Bitselect, V128AnyTrue,
                    I8x16Abs, I8x16Neg, I8x16Popcnt, I8x16AllTrue, I8x16Bitmask, I8x16NarrowI16x8S, I8x16NarrowI16x8U,
                    I8x16Shl, I8x16ShrS, I8x16ShrU, I8x16Add, I8x16AddSatS, I8x16AddSatU, I8x16Sub, I8x16SubSatS,
                    I8x16SubSatU, I8x16MinS, I8x16MinU, I8x16MaxS, I8x16MaxU, I8x16RoundingAverageU,
                    I16x8ExtAddPairwiseI8x16S, I16x8ExtAddPairwiseI8x16U, I16x8Abs, I16x8Neg, I16x8Q15MulrSatS,
                    I16x8AllTrue, I16x8Bitmask, I16x8NarrowI32x4S, I16x8NarrowI32x4U, I16x8ExtendLowI8x16S,
                    I16x8ExtendHighI8x16S, I16x8ExtendLowI8x16U, I16x8ExtendHighI8x16U, I16x8Shl, I16x8ShrS, I16x8ShrU,
                    I16x8Add, I16x8AddSatS, I16x8AddSatU, I16x8Sub, I16x8SubSatS, I16x8SubSatU, I16x8Mul, I16x8MinS,
                    I16x8MinU, I16x8MaxS, I16x8MaxU, I16x8RoundingAverageU, I16x8ExtMulLowI8x16S,
                    I16x8ExtMulHighI8x16S, I16x8ExtMulLowI8x16U, I16x8ExtMulHighI8x16U, I32x4ExtAddPairwiseI16x8S,
                    I32x4ExtAddPairwiseI16x8U, I32x4Abs, I32x4Neg, I32x4AllTrue, I32x4Bitmask, I32x4ExtendLowI16x8S,
                    I32x4ExtendHighI16x8S, I32x4ExtendLowI16x8U, I32x4ExtendHighI16x8U, I32x4Shl, I32x4ShrS, I32x4ShrU,
                    I32x4Add, I32x4Sub, I32x4Mul, I32x4MinS, I32x4MinU, I32x4MaxS, I32x4MaxU, I32x4DotI16x8S,
                    I32x4ExtMulLowI16x8S, I32x4ExtMulHighI16x8S, I32x4ExtMulLowI16x8U, I32x4ExtMulHighI16x8U, I64x2Abs,
                    I64x2Neg, I64x2AllTrue, I64x2Bitmask, I64x2ExtendLowI32x4S, I64x2ExtendHighI32x4S,
                    I64x2ExtendLowI32x4U, I64x2ExtendHighI32x4U, I64x2Shl, I64x2ShrS, I64x2ShrU, I64x2Add, I64x2Sub,
                    I64x2Mul, I64x2ExtMulLowI32x4S, I64x2ExtMulHighI32x4S, I64x2ExtMulLowI32x4U, I64x2ExtMulHighI32x4U,
                    F32x4Ceil, F32x4Floor, F32x4Trunc, F32x4Nearest, F32x4Abs, F32x4Neg, F32x4Sqrt, F32x4Add, F32x4Sub,
                    F32x4Mul, F32x4Div, F32x4Min, F32x4Max, F32x4PMin, F32x4PMax, F64x2Ceil, F64x2Floor, F64x2Trunc,
                    F64x2Nearest, F64x2Abs, F64x2Neg, F64x2Sqrt, F64x2Add, F64x2Sub, F64x2Mul, F64x2Div, F64x2Min,
                    F64x2Max, F64x2PMin, F64x2PMax, I32x4TruncSatF32x4S, I32x4TruncSatF32x4U, F32x4ConvertI32x4S,
                    F32x4ConvertI32x4U, I32x4TruncSatF64x2SZero, I32x4TruncSatF64x2UZero, F64x2ConvertLowI32x4S,
                    F64x2ConvertLowI32x4U, F32x4DemoteF64x2Zero, F64x2PromoteLowF32x4, I8x16RelaxedSwizzle,
                    I32x4RelaxedTruncSatF32x4S, I32x4RelaxedTruncSatF32x4U, I32x4RelaxedTruncSatF64x2SZero,
                    I32x4RelaxedTruncSatF64x2UZero, F32x4Fma, F32x4Fms, F64x2Fma, F64x2Fms, I8x16LaneSelect,
                    I16x8LaneSelect, I32x4LaneSelect, I64x2LaneSelect, F32x4RelaxedMin, F32x4RelaxedMax,
                    F64x2RelaxedMin, F64x2RelaxedMax
                )
            ) => error!("SIMD extension not supported {:?}", unsupported),

            _ => unimplemented!(),
        };
    }

    Ok(stack + 4)
}