// Copyright 2021-2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use crate::{
    machine::{Function, InboxIdentifier},
    programs::PolyHostData,
    value::{ArbValueType, FunctionType, IntegerValType},
    wavm::{IBinOpType, Instruction, Opcode},
};

pub fn get_host_impl(module: &str, name: &str) -> eyre::Result<Function> {
    let mut out = vec![];
    let ty;

    macro_rules! opcode {
        ($opcode:expr) => {
            out.push(Instruction::simple($opcode))
        };
        ($opcode:expr, $value:expr) => {
            out.push(Instruction::with_data($opcode, $value))
        };
    }

    use ArbValueType::*;
    use Opcode::*;
    match (module, name) {
        ("env", "wavm_caller_load8") => {
            ty = FunctionType::new(vec![I32], vec![I32]);
            opcode!(LocalGet, 0);
            opcode!(CallerModuleInternalCall, WAVM_CALLER_LOAD8);
        }
        ("env", "wavm_caller_load32") => {
            ty = FunctionType::new(vec![I32], vec![I32]);
            opcode!(LocalGet, 0);
            opcode!(CallerModuleInternalCall, WAVM_CALLER_LOAD32);
        }
        ("env", "wavm_caller_store8") => {
            ty = FunctionType::new(vec![I32; 2], vec![]);
            opcode!(LocalGet, 0);
            opcode!(LocalGet, 1);
            opcode!(CallerModuleInternalCall, WAVM_CALLER_STORE8);
        }
        ("env", "wavm_caller_store32") => {
            ty = FunctionType::new(vec![I32; 2], vec![]);
            opcode!(LocalGet, 0);
            opcode!(LocalGet, 1);
            opcode!(CallerModuleInternalCall, WAVM_CALLER_STORE32);
        }
        ("env", "wavm_get_globalstate_bytes32") => {
            ty = FunctionType::new(vec![I32; 2], vec![]);
            opcode!(LocalGet, 0);
            opcode!(LocalGet, 1);
            opcode!(GetGlobalStateBytes32);
        }
        ("env", "wavm_set_globalstate_bytes32") => {
            ty = FunctionType::new(vec![I32; 2], vec![]);
            opcode!(LocalGet, 0);
            opcode!(LocalGet, 1);
            opcode!(SetGlobalStateBytes32);
        }
        ("env", "wavm_get_globalstate_u64") => {
            ty = FunctionType::new(vec![I32], vec![I64]);
            opcode!(LocalGet, 0);
            opcode!(GetGlobalStateU64);
        }
        ("env", "wavm_set_globalstate_u64") => {
            ty = FunctionType::new(vec![I32, I64], vec![]);
            opcode!(LocalGet, 0);
            opcode!(LocalGet, 1);
            opcode!(SetGlobalStateU64);
        }
        ("env", "wavm_read_pre_image") => {
            ty = FunctionType::new(vec![I32; 2], vec![I32]);
            opcode!(LocalGet, 0);
            opcode!(LocalGet, 1);
            opcode!(ReadPreImage);
        }
        ("env", "wavm_read_inbox_message") => {
            ty = FunctionType::new(vec![I64, I32, I32], vec![I32]);
            opcode!(LocalGet, 0);
            opcode!(LocalGet, 1);
            opcode!(LocalGet, 2);
            opcode!(ReadInboxMessage, InboxIdentifier::Sequencer as u64);
        }
        ("env", "wavm_read_delayed_inbox_message") => {
            ty = FunctionType::new(vec![I64, I32, I32], vec![I32]);
            opcode!(LocalGet, 0);
            opcode!(LocalGet, 1);
            opcode!(LocalGet, 2);
            opcode!(ReadInboxMessage, InboxIdentifier::Delayed as u64);
        }
        ("env", "wavm_get_caller_module") => {
            ty = FunctionType::new(vec![], vec![I32]);
            opcode!(CallerModule);
        }
        ("env", "wavm_link_program") => {
            ty = FunctionType::new(vec![I32], vec![I32]);
            opcode!(LocalGet, 0);
            opcode!(LinkModule);
        }
        ("env", "wavm_prep_program") => {
            ty = FunctionType::new(vec![I32, I32, I64], vec![]);
            opcode!(LocalGet, 2); // gas left
            opcode!(I32Const, 0); // gas status
            opcode!(LocalGet, 0); // the module
            opcode!(LocalGet, 1); // the internals offset
            opcode!(I32Const, POLY_WAVM_SET_GAS); // the relative position of poly_wavm_set_gas
            opcode!(IBinOp(IntegerValType::I32, IBinOpType::Add)); // absolute position of poly_wavm_set_gas
            opcode!(CrossModuleDynamicCall); // consumes module and func, forwarding the others
        }
        ("env", "wavm_call_program") => {
            ty = FunctionType::new(vec![I32, I32, I32], vec![I32]);
            opcode!(LocalGet, 2); // the args len
            opcode!(LocalGet, 0); // the module
            opcode!(LocalGet, 1); // the function
            opcode!(CrossModuleDynamicCall) // consumes module and func, forwarding the args len
        }
        ("env", "wavm_halt_and_set_finished") => {
            ty = FunctionType::default();
            opcode!(HaltAndSetFinished);
        }
        ("env", "poly_wavm_gas_left") => {
            ty = FunctionType::new(vec![], vec![I64]);
            opcode!(CallerModuleInternalCall, POLY_WAVM_GAS_LEFT);
        }
        ("env", "poly_wavm_gas_status") => {
            ty = FunctionType::new(vec![], vec![I32]);
            opcode!(CallerModuleInternalCall, POLY_WAVM_GAS_STATUS);
        }
        ("env", "poly_wavm_set_gas") => {
            ty = FunctionType::new(vec![I64, I32], vec![]);
            opcode!(LocalGet, 0);
            opcode!(LocalGet, 1);
            opcode!(CallerModuleInternalCall, POLY_WAVM_SET_GAS);
        }
        ("dynamic_host", "wavm_read_program_gas_left") => {
            ty = FunctionType::new(vec![I32, I32], vec![I64]);
            opcode!(LocalGet, 0); // the module
            opcode!(LocalGet, 1); // the internals offset
            opcode!(I32Const, POLY_WAVM_GAS_LEFT); // the relative position of poly_wavm_gas_left
            opcode!(IBinOp(IntegerValType::I32, IBinOpType::Add)); // absolute position of poly_wavm_gas_left
            opcode!(CrossModuleDynamicCall);
        }
        ("dynamic_host", "wavm_read_program_gas_status") => {
            ty = FunctionType::new(vec![I32, I32], vec![I32]);
            opcode!(LocalGet, 0); // the module
            opcode!(LocalGet, 1); // the internals offset
            opcode!(I32Const, POLY_WAVM_GAS_STATUS); // the relative position of poly_wavm_gas_status
            opcode!(IBinOp(IntegerValType::I32, IBinOpType::Add)); // absolute position of poly_wavm_gas_status
            opcode!(CrossModuleDynamicCall);
        }
        ("dynamic_host", "wavm_read_program_depth_left") => {
            ty = FunctionType::new(vec![I32, I32], vec![I32]);
            opcode!(LocalGet, 0); // the module
            opcode!(LocalGet, 1); // the internals offset
            opcode!(I32Const, POLY_WAVM_DEPTH_LEFT); // the relative position of poly_wavm_depth_left
            opcode!(IBinOp(IntegerValType::I32, IBinOpType::Add)); // absolute position of poly_wavm_depth_left
            opcode!(CrossModuleDynamicCall);
        }
        _ => eyre::bail!("Unsupported import of {:?} {:?}", module, name),
    }

    let append = |code: &mut Vec<Instruction>| {
        code.extend(out);
        Ok(())
    };

    Function::new(&[], append, ty, &[])
}

const WAVM_CALLER_LOAD8: u64 = 0;
const WAVM_CALLER_LOAD32: u64 = 1;
const WAVM_CALLER_STORE8: u64 = 2;
const WAVM_CALLER_STORE32: u64 = 3;
const POLY_WAVM_GAS_LEFT: u64 = 4;
const POLY_WAVM_GAS_STATUS: u64 = 5;
const POLY_WAVM_DEPTH_LEFT: u64 = 6;
const POLY_WAVM_SET_GAS: u64 = 7;

/// Adds internal functions to a module.
/// Note: the order of the functions matters and must match the dynamic calls in the wavm hostio impls
pub fn add_internal_funcs(
    funcs: &mut Vec<Function>,
    func_types: &mut Vec<FunctionType>,
    poly_host: Option<PolyHostData>,
) {
    use ArbValueType::*;
    use Opcode::*;

    fn code_func(code: Vec<Instruction>, ty: FunctionType) -> Function {
        let mut wavm = vec![Instruction::simple(InitFrame)];
        wavm.extend(code);
        wavm.push(Instruction::simple(Return));
        Function::new_from_wavm(wavm, ty, Vec::new())
    }

    fn op_func(opcode: Opcode, ty: FunctionType) -> Function {
        code_func(vec![Instruction::simple(opcode)], ty)
    }

    macro_rules! host {
        ($ins:expr, $outs:expr) => {{
            let ty = FunctionType::new($ins, $outs);
            func_types.push(ty.clone());
            ty
        }};
    }

    funcs.push(op_func(
        MemoryLoad {
            ty: I32,
            bytes: 1,
            signed: false,
        },
        host!(vec![I32], vec![I32]), // wavm_caller_load8
    ));
    funcs.push(op_func(
        MemoryLoad {
            ty: I32,
            bytes: 4,
            signed: false,
        },
        host!(vec![I32], vec![I32]), // wavm_caller_load32
    ));
    funcs.push(op_func(
        MemoryStore { ty: I32, bytes: 1 },
        host!(vec![I32], vec![I32]), // wavm_caller_store8
    ));
    funcs.push(op_func(
        MemoryStore { ty: I32, bytes: 4 },
        host!(vec![I32], vec![I32]), // wavm_caller_store32
    ));

    if let Some(poly_host) = poly_host {
        let (gas, status, depth) = poly_host.globals();
        funcs.push(code_func(
            vec![Instruction::with_data(GlobalGet, gas)],
            host!(vec![], vec![I64]), // poly_wavm_gas_left
        ));
        funcs.push(code_func(
            vec![Instruction::with_data(GlobalGet, status)],
            host!(vec![], vec![I32]), // poly_wavm_gas_status
        ));
        funcs.push(code_func(
            vec![Instruction::with_data(GlobalGet, depth)],
            host!(vec![], vec![I32]), // poly_wavm_depth_left
        ));
        funcs.push(code_func(
            vec![
                Instruction::with_data(GlobalSet, status),
                Instruction::with_data(GlobalSet, gas),
            ],
            host!(vec![I64, I32], vec![]), // poly_wavm_set_gas
        ));
    }
}
