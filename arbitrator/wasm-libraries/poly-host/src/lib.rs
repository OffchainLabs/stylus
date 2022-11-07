// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

mod gas;

use hashbrown::HashMap;
use arbutil::color;

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
    gas_price: u64,
}

#[no_mangle]
pub unsafe extern "C" fn poly_host__read_args(ptr: usize) {
    let module = wavm_get_caller_module();
    let Some(program) = PROGRAMS.get(&module) else {
        panic!("missing program {}", color::red(module));
    };
    println!(
        "read args {} {} {}",
        module,
        program.args.len(),
        String::from_utf8_lossy(&program.args)
    );
    wavm_util::write_slice(&program.args, ptr);
}

#[no_mangle]
pub unsafe extern "C" fn poly_host__return_data(len: usize, ptr: usize) {
    let module = wavm_get_caller_module();
    let Some(program) = PROGRAMS.get_mut(&module) else {
        panic!("missing program {}", color::red(module));
    };

    let evm_words = |count: u64| count.saturating_add(31) / 32;
    let evm_gas = evm_words(len as u64).saturating_mul(3); // each byte is 3 evm gas per evm word
    gas::buy_evm_gas(evm_gas, program.gas_price);

    program.outs = wavm_util::read_slice(ptr, len);
    println!("return data {} {}", program.outs.len(), hex::encode(&program.outs));
}

#[no_mangle]
pub unsafe extern "C" fn poly_host__write_args(module: u32, ptr: usize, len: usize, gas_price: u64) -> usize {
    let args = wavm_util::read_slice(ptr, len);
    let outs = vec![];
    let program = Program { args, outs, gas_price };
    let data = program.args.as_ptr();
    PROGRAMS.insert(module, program);
    data as usize
}

#[no_mangle]
pub unsafe extern "C" fn poly_host__read_output_len(module: u32) -> usize {
    match PROGRAMS.get_mut(&module) {
        Some(program) => program.outs.len(),
        None => panic!("no program"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn poly_host__read_output_ptr(module: u32) -> usize {
    match PROGRAMS.get_mut(&module) {
        Some(program) => program.outs.as_ptr() as usize,
        None => panic!("no program"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn poly_host__read_output(module: u32, output: *mut u8) {
    let Some(program) = PROGRAMS.get_mut(&module) else {
        panic!("missing program {}", color::red(module));
    };
    wavm_util::write_slice(&program.outs, output as usize);
}

#[no_mangle]
pub unsafe extern "C" fn poly_host__clear_program(module: u32) {
    PROGRAMS.remove(&module);
}
