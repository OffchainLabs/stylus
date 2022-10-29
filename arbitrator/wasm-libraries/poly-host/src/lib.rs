// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use core::slice;

use go_abi::GoStack;

mod gas;
mod util;

pub use gas::set_gas_price;
use hashbrown::HashMap;

extern "C" {
    fn wavm_get_caller_module() -> u32;
}

static mut PROGRAMS: HashMap<u32, Program> =
    HashMap::with_hasher(hashbrown::hash_map::DefaultHashBuilder::with_seeds(
        0x243f_6a88_85a3_08d3,
        0x1319_8a2e_0370_7344,
        0xa409_3822_299f_31d0,
        0x082e_fa98_ec4e_6c89,
    ));

struct Program {
    args: Vec<u8>,
    outs: Vec<u8>,
}

#[no_mangle]
pub unsafe extern "C" fn poly_host__read_args(ptr: usize) {
    //let module = wavm_get_caller_module();
    let program = match PROGRAMS.get(&0) {
        Some(program) => program,
        None => return,
    };
    println!(
        "read args {} {}",
        program.args.len(),
        String::from_utf8_lossy(&program.args)
    );
    util::write_slice(&program.args, ptr);
}

#[no_mangle]
pub unsafe extern "C" fn poly_host__return_data(len: usize, ptr: usize) {
    //let module = wavm_get_caller_module();
    let program = match PROGRAMS.get_mut(&0) {
        Some(program) => program,
        None => return,
    };

    let evm_words = |count: u64| count.saturating_add(31) / 32;
    let evm_gas = evm_words(len as u64).saturating_mul(3); // each byte is 3 evm gas per evm word
    gas::buy_evm_gas(evm_gas);

    program.outs = util::read_slice(ptr, len);
    println!("return data {}", hex::encode(&program.outs));
}

#[no_mangle]
pub unsafe extern "C" fn poly_host__allocate_args(bytes: usize) -> usize {
    let mut args = Vec::with_capacity(bytes);
    args.set_len(bytes);

    let outs = vec![];
    let program = Program { args, outs };
    let data = program.args.as_ptr();
    PROGRAMS.insert(0, program);
    data as usize
}

#[no_mangle]
pub unsafe extern "C" fn poly_host__read_output_len() -> usize {
    match PROGRAMS.get_mut(&0) {
        Some(program) => program.outs.len(),
        None => panic!("no program"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn poly_host__read_output_ptr() -> usize {
    match PROGRAMS.get_mut(&0) {
        Some(program) => program.outs.as_ptr() as usize,
        None => panic!("no program"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_polyglotCheck(
    sp: GoStack,
) {
    // func (wasm []byte) (status uint64, output *byte, outlen, outcap uint64)
    let wasm_ptr = sp.read_u64(0);
    let wasm_len = sp.read_u64(1);
    let wasm = go_abi::read_slice(wasm_ptr, wasm_len);

    
}

#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_polyglotCall(
    sp: GoStack,
) {
    // func (wasm, calldata []byte, gas_price uint64, output *byte, outlen, outcap, gas *uint64) (status uint64)
}

#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_polyglotFree(
    sp: GoStack,
) {
    // func(output *byte, outlen, outcap uint64)
    let ptr = usize::try_from(sp.read_u64(0)).expect("Go pointer didn't fit in usize") as *mut u8;
    let len = sp.read_u64(1).try_into().unwrap();
    let cap = sp.read_u64(2).try_into().unwrap();

    let vec = Vec::from_raw_parts(ptr, len, cap);
    std::mem::drop(vec)
}
