// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use crate::{evm_api::ApiCaller, Program, PROGRAMS};
use arbutil::{
    evm::{js::JsEvmApi, user::UserOutcomeKind, EvmData},
    format::DebugBytes,
    heapify, wavm, Bytes32,
};
use go_abi::GoStack;
use prover::{
    machine::Module,
    programs::config::{PricingParams, StylusConfig},
};
use std::mem;

// these hostio methods allow the replay machine to modify itself
#[link(wasm_import_module = "hostio")]
extern "C" {
    fn wavm_link_module(hash: *const MemoryLeaf) -> u32;
    fn wavm_unlink_module();
}

// these dynamic hostio methods allow introspection into user modules
#[link(wasm_import_module = "hostio")]
extern "C" {
    fn program_set_ink(module: u32, ink: u64);
    fn program_set_stack(module: u32, stack: u32);
    fn program_ink_left(module: u32) -> u64;
    fn program_ink_status(module: u32) -> u32;
    fn program_stack_left(module: u32) -> u32;
    fn program_call_main(module: u32, args_len: usize) -> u32;
}

#[repr(C, align(256))]
struct MemoryLeaf([u8; 32]);

/// Instruments and "activates" a user wasm, producing a unique module hash.
///
/// Note that this operation costs gas and is limited by the amount supplied via the `gas` pointer.
/// The amount left is written back at the end of the call.
///
/// # Safety
///
/// The `modHash` and `gas` pointers must not be null.
///
/// The Go compiler expects the call to take the form
///     λ(wasm []byte, pageLimit, version u16, debug u32, modHash *hash, gas *u64) (footprint u16, err *Vec<u8>)
///
/// These values are placed on the stack as follows
///     || wasm... || pageLimit | version | debug || modhash ptr || gas ptr || footprint | 6 pad || err ptr ||
///
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_activateProgramRustImpl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);
    let wasm = sp.read_go_slice_owned();
    let page_limit = sp.read_u16();
    let version = sp.read_u16();
    let debug = sp.read_bool32();
    let module_hash = sp.read_go_ptr();
    let gas = sp.read_go_ptr();

    macro_rules! error {
        ($msg:expr, $error:expr) => {{
            let error = $error.wrap_err($msg).debug_bytes();
            wavm::caller_store64(gas, 0);
            wavm::write_bytes32(module_hash, Bytes32::default());
            sp.skip_space();
            sp.write_ptr(heapify(error));
            return;
        }};
    }

    let gas_left = &mut wavm::caller_load64(gas);
    let (module, pages) = match Module::activate(&wasm, version, page_limit, debug, gas_left) {
        Ok(data) => data,
        Err(error) => error!("failed to activate", error),
    };
    wavm::caller_store64(gas, *gas_left);
    wavm::write_bytes32(module_hash, module.hash());
    sp.write_u16(pages).skip_space();
    sp.write_nullptr();
}

/// Links and executes a user wasm.
///
/// # Safety
///
/// The Go compiler expects the call to take the form
///     λ(moduleHash *[32]byte, calldata []byte, params *Configs, evmApi []byte, evmData: *EvmData, gas *u64) (
///         status byte, out *Vec<u8>,
///     )
///
/// These values are placed on the stack as follows
///     || modHash || calldata... || params || evmApi... || evmData || gas || status | 7 pad | out ptr ||
///
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_callProgramRustImpl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);
    let compiled_hash = wavm::read_bytes32(sp.read_go_ptr());
    let calldata = sp.read_go_slice_owned();
    let config: StylusConfig = sp.unbox();
    let evm_api = JsEvmApi::new(sp.read_go_slice_owned(), ApiCaller::new());
    let evm_data: EvmData = sp.unbox();
    let gas = sp.read_go_ptr();

    // buy ink
    let pricing = config.pricing;
    let ink = pricing.gas_to_ink(wavm::caller_load64(gas));

    // link the program and ready its instrumentation
    let module = wavm_link_module(&MemoryLeaf(*compiled_hash));
    program_set_ink(module, ink);
    program_set_stack(module, config.max_depth);

    // provide arguments
    let args_len = calldata.len();
    PROGRAMS.push(Program::new(calldata, evm_api, evm_data, config));

    // call the program
    let go_stack = sp.save_stack();
    let status = program_call_main(module, args_len);
    let outs = PROGRAMS.pop().unwrap().into_outs();
    sp.restore_stack(go_stack);

    /// cleans up and writes the output
    macro_rules! finish {
        ($status:expr, $ink_left:expr) => {
            finish!($status, std::ptr::null::<u8>(), $ink_left);
        };
        ($status:expr, $outs:expr, $ink_left:expr) => {{
            sp.write_u8($status as u8).skip_space();
            sp.write_ptr($outs);
            wavm::caller_store64(gas, pricing.ink_to_gas($ink_left));
            wavm_unlink_module();
            return;
        }};
    }

    // check if instrumentation stopped the program
    use UserOutcomeKind::*;
    if program_ink_status(module) != 0 {
        finish!(OutOfInk, 0);
    }
    if program_stack_left(module) == 0 {
        finish!(OutOfStack, 0);
    }

    // the program computed a final result
    let ink_left = program_ink_left(module);
    finish!(status, heapify(outs), ink_left)
}

/// Reads the length of a rust `Vec`
///
/// # Safety
///
/// The Go compiler expects the call to take the form
///     λ(vec *Vec<u8>) (len u32)
///
/// These values are placed on the stack as follows
///     || vec ptr || len u32 | pad 4 ||
///
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_readRustVecLenImpl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);
    let vec: &Vec<u8> = &*sp.read_ptr();
    sp.write_u32(vec.len() as u32);
}

/// Copies the contents of a rust `Vec` into a go slice, dropping it in the process
///
/// # Safety
///
/// The Go compiler expects the call to take the form
///     λ(vec *Vec<u8>, dest []byte)
///
/// These values are placed on the stack as follows
///     || vec ptr || dest... ||
///
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_rustVecIntoSliceImpl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);
    let vec: Vec<u8> = sp.unbox();
    let ptr: *mut u8 = sp.read_ptr_mut();
    wavm::write_slice(&vec, ptr as u64);
    mem::drop(vec)
}

/// Creates a `StylusConfig` from its component parts.
///
/// # Safety
///
/// The Go compiler expects the call to take the form
///     λ(version u16, maxDepth, inkPrice u32, debugMode u32) *StylusConfig
///
/// The values are placed on the stack as follows
///     || version | 2 garbage bytes | max_depth || ink_price | debugMode || result ptr ||
///
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_rustConfigImpl(
    sp: usize,
) {
    let mut sp = GoStack::new(sp);

    let config = StylusConfig {
        version: sp.read_u16(),
        max_depth: sp.skip_u16().read_u32(),
        pricing: PricingParams {
            ink_price: sp.read_u32(),
        },
    };
    sp.skip_u32(); // skip debugMode
    sp.write_ptr(heapify(config));
}

/// Creates an `EvmData` from its component parts.
///
/// # Safety
///
/// The Go compiler expects the call to take the form
///     λ(
///         blockBasefee *[32]byte, chainid u64, blockCoinbase *[20]byte, blockGasLimit,
///         blockNumber, blockTimestamp u64, contractAddress, msgSender *[20]byte,
///         msgValue, txGasPrice *[32]byte, txOrigin *[20]byte, reentrant u32,
///     ) -> *EvmData
///
/// These values are placed on the stack as follows
///     || baseFee || chainid || coinbase || gas limit || block number || timestamp || address ||
///     || sender || value || gas price || origin || reentrant | 4 pad || data ptr ||
///
#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_rustEvmDataImpl(
    sp: usize,
) {
    use wavm::{read_bytes20, read_bytes32};
    let mut sp = GoStack::new(sp);
    let evm_data = EvmData {
        block_basefee: read_bytes32(sp.read_go_ptr()),
        chainid: sp.read_u64(),
        block_coinbase: read_bytes20(sp.read_go_ptr()),
        block_gas_limit: sp.read_u64(),
        block_number: sp.read_u64(),
        block_timestamp: sp.read_u64(),
        contract_address: read_bytes20(sp.read_go_ptr()),
        msg_sender: read_bytes20(sp.read_go_ptr()),
        msg_value: read_bytes32(sp.read_go_ptr()),
        tx_gas_price: read_bytes32(sp.read_go_ptr()),
        tx_origin: read_bytes20(sp.read_go_ptr()),
        reentrant: sp.read_u32(),
        return_data_len: 0,
        tracing: false,
    };
    sp.skip_space();
    sp.write_ptr(heapify(evm_data));
}
