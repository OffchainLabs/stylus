// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use crate::{Program, PROGRAMS};
use arbutil::{heapify, wavm};
use fnv::FnvHashMap as HashMap;
use go_abi::GoStack;
use prover::{
    programs::{config::StylusConfig, config::EvmContext, run::UserOutcomeKind},
    Machine,
    utils::{Bytes20, Bytes32},
};
use std::{mem, path::Path, sync::Arc};

// these hostio methods allow the replay machine to modify itself
#[link(wasm_import_module = "hostio")]
extern "C" {
    fn link_module(hash: *const MemoryLeaf) -> u32;
    fn unlink_module();
}

// these dynamic hostio methods allow introspection into user modules
#[link(wasm_import_module = "hostio")]
extern "C" {
    fn program_set_gas(module: u32, internals: u32, gas: u64);
    fn program_set_stack(module: u32, internals: u32, stack: u32);
    fn program_gas_left(module: u32, internals: u32) -> u64;
    fn program_gas_status(module: u32, internals: u32) -> u32;
    fn program_stack_left(module: u32, internals: u32) -> u32;
    fn program_call_main(module: u32, main: u32, args_len: usize) -> u32;
}

#[repr(C, align(256))]
struct MemoryLeaf([u8; 32]);

/// Compiles and instruments user wasm.
/// Safety: λ(wasm []byte, version u32) (machine *Machine, err *Vec<u8>)
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_compileUserWasmRustImpl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);
    let wasm = sp.read_go_slice_owned();
    let config = StylusConfig::version(sp.read_u32());
    sp.skip_space();

    macro_rules! error {
        ($msg:expr, $error:expr) => {{
            let error = format!("{}: {:?}", $msg, $error).as_bytes().to_vec();
            sp.write_nullptr();
            sp.write_ptr(heapify(error));
            return;
        }};
    }

    let mut bin = match prover::binary::parse(&wasm, Path::new("user")) {
        Ok(bin) => bin,
        Err(err) => error!("failed to parse user program", err),
    };
    let stylus_data = match bin.instrument(&config) {
        Ok(stylus_data) => stylus_data,
        Err(err) => error!("failed to instrument user program", err),
    };

    let forward = include_bytes!("../../../../target/machines/latest/forward_stub.wasm");
    let forward = prover::binary::parse(forward, Path::new("forward")).unwrap();

    let machine = Machine::from_binaries(
        &[forward],
        bin,
        false,
        false,
        false,
        prover::machine::GlobalState::default(),
        HashMap::default(),
        Arc::new(|_, _| panic!("user program tried to read preimage")),
        Some(stylus_data),
    );
    let machine = match machine {
        Ok(machine) => machine,
        Err(err) => error!("failed to instrument user program", err),
    };
    sp.write_ptr(heapify(machine));
    sp.write_nullptr();
}

/// Links and executes a user wasm.
/// Safety: λ(mach *Machine, data []byte, params *StylusConfig, evmContext *EvmContext, gas *u64, root *[32]byte) (status byte, out *Vec<u8>)
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_callUserWasmRustImpl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);
    let machine: Machine = *Box::from_raw(sp.read_ptr_mut());
    let calldata = sp.read_go_slice_owned();
    let config: StylusConfig = *Box::from_raw(sp.read_ptr_mut());
    let _: EvmContext = *Box::from_raw(sp.read_ptr_mut());

    // buy wasm gas. If free, provide a virtually limitless amount
    let pricing = config.pricing;
    let evm_gas = sp.read_go_ptr();
    let wasm_gas = pricing
        .evm_to_wasm(wavm::caller_load64(evm_gas))
        .unwrap_or(u64::MAX);

    // compute the module root, or accept one from the caller
    let root = sp.read_go_ptr();
    let root = (root != 0).then(|| wavm::read_bytes32(root as u64));
    let module = root.unwrap_or_else(|| machine.main_module_hash().0);
    let (main, internals) = machine.program_info();

    // link the program and ready its instrumentation
    let module = link_module(&MemoryLeaf(module));
    program_set_gas(module, internals, wasm_gas);
    program_set_stack(module, internals, config.depth.max_depth);

    // provide arguments
    let args_len = calldata.len();
    PROGRAMS.push(Program::new(calldata, config.pricing));

    // call the program
    let status = program_call_main(module, main, args_len);
    let outs = PROGRAMS.pop().unwrap().into_outs();

    /// cleans up and writes the output
    macro_rules! finish {
        ($status:expr, $gas_left:expr) => {
            finish!($status, std::ptr::null::<u8>(), $gas_left);
        };
        ($status:expr, $outs:expr, $gas_left:expr) => {{
            sp.write_u8($status as u8).skip_space();
            sp.write_ptr($outs);
            if pricing.wasm_gas_price != 0 {
                wavm::caller_store64(evm_gas, pricing.wasm_to_evm($gas_left));
            }
            unlink_module();
            return;
        }};
    }

    // check if instrumentation stopped the program
    use UserOutcomeKind::*;
    if program_gas_status(module, internals) != 0 {
        finish!(OutOfGas, 0);
    }
    if program_stack_left(module, internals) == 0 {
        finish!(OutOfStack, 0);
    }

    // the program computed a final result
    let gas_left = program_gas_left(module, internals);
    match status {
        0 => finish!(Success, heapify(outs), gas_left),
        _ => finish!(Revert, heapify(outs), gas_left),
    };
}

/// Reads the length of a rust `Vec`
/// Safety: λ(vec *Vec<u8>) (len u32)
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_readRustVecLenImpl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);
    let vec: &Vec<u8> = &*sp.read_ptr();
    sp.write_u32(vec.len() as u32);
}

/// Copies the contents of a rust `Vec` into a go slice, dropping it in the process
/// Safety: λ(vec *Vec<u8>, dest []byte)
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_rustVecIntoSliceImpl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);
    let vec: Vec<u8> = *Box::from_raw(sp.read_ptr_mut());
    let ptr: *mut u8 = sp.read_ptr_mut();
    wavm::write_slice(&vec, ptr as u64);
    mem::drop(vec)
}

/// Creates a `StylusConfig` from its component parts.
/// Safety: λ(version, maxDepth u32, wasmGasPrice, hostioCost u64) *StylusConfig
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_rustConfigImpl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);
    let version = sp.read_u32();

    let mut config = StylusConfig::version(version);
    config.depth.max_depth = sp.read_u32();
    config.pricing.wasm_gas_price = sp.read_u64();
    config.pricing.hostio_cost = sp.read_u64();
    sp.write_ptr(heapify(config));
}

/// Returns a uint32_t from a bool
/// Safety: λ(b bool) u32
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_boolToRustIntImpl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);
    let val: u8 = sp.read_u8();
    sp.write_u32(val as u32);
}

/// Creates a `Bytes20` from an address
/// Safety: λ(addr common.Address) *Bytes20
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_addressToRustBytes20Impl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);

    let bytes = sp.read_go_ptr();
    let bytes = wavm::read_bytes20(bytes as u64);
    sp.write_ptr(heapify(bytes));
}

/// Creates a `Bytes32` from a Hash
/// Safety: λ(hash common.Hash) *Bytes32
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_hashToRustBytes32Impl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);

    let hash = sp.read_go_ptr();
    let hash = wavm::read_bytes32(hash as u64);
    sp.write_ptr(heapify(hash));
}

/// Creates an `EvmContext` from its component parts.
/// Safety: λ(readOnly u32, origin *Bytes20, gasPrice u64, coinbase *Bytes20, gasLimit u64, time *Bytes32, difficulty *Bytes32, baseFee u64, random *Bytes32) *StylusConfig
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_rustEvmContextImpl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);

    let mut context = EvmContext::default();
    context.read_only = sp.read_u32() != 0;
    let origin: Bytes20 = *Box::from_raw(sp.read_ptr_mut());
    context.origin = origin;
    context.gas_price = sp.read_u64();
    let coinbase: Bytes20 = *Box::from_raw(sp.read_ptr_mut());
    context.coinbase = coinbase;
    context.gas_limit = sp.read_u64();
    let time: Bytes32 = *Box::from_raw(sp.read_ptr_mut());
    context.time = time;
    let difficulty: Bytes32 = *Box::from_raw(sp.read_ptr_mut());
    context.difficulty = difficulty;
    context.base_fee = sp.read_u64();
    let random: Bytes32 = *Box::from_raw(sp.read_ptr_mut());
    context.random = random;

    sp.write_ptr(heapify(context));
}
