// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use core::slice;

use go_abi::GoStack;
use prover::{programs::PolyglotConfig, Machine};

extern "C" {
    fn wavm_link_program(hash: *const MemoryLeaf) -> u32;
    fn wavm_prep_program(module: u32, internals: u32, gas: u64);
    fn wavm_call_program(module: u32, main: u32, args_len: usize) -> u32;
}

#[link(wasm_import_module = "dynamic_host")]
extern "C" {
    fn wavm_read_program_gas_left(module: u32, internals: u32) -> u64;
    fn wavm_read_program_gas_status(module: u32, internals: u32) -> u32;
    fn wavm_read_program_depth_left(module: u32, internals: u32) -> u32;
}

#[link(wasm_import_module = "poly_host")]
extern "C" {
    fn write_args(module: u32, args: *const u8, args_len: usize, gas_price: u64) -> usize;
    fn read_output(module: u32, output: *mut u8);
    fn read_output_len(module: u32) -> usize;
    fn clear_program(module: u32);
}

#[repr(C, align(256))]
struct MemoryLeaf([u8; 32]);

#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_polyglotCheck(
    sp: GoStack,
) {
    // func (wasm []byte) (status uint64, output *byte, outlen, outcap uint64)
    const IN_PTR: usize = 0;
    const IN_LEN: usize = 1;
    const STATUS: usize = 3;
    const OUTPTR: usize = 4;
    const OUTLEN: usize = 5;
    const OUTCAP: usize = 6;

    let wasm = read_go_slice(sp, IN_PTR, IN_LEN);

    let config = PolyglotConfig::default();
    let machine = Machine::from_polyglot_binary(&wasm, true, &config);
    let status = if machine.is_err() { 1 } else { 0 };
    let output = machine.map_err(|e| e.to_string()).err().unwrap_or_default();
    let output = output.as_bytes().to_vec();

    sp.write_u64(STATUS, status);
    write_output(sp, &output, OUTPTR, OUTLEN, OUTCAP);
    std::mem::forget(output);
}

#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_polyglotCall(
    sp: GoStack,
) {
    // func (wasm, calldata []byte, gas_price uint64, gas *uint64) (status uint64, output *byte, outlen, outcap uint64)
    const WASM_PTR: usize = 0;
    const WASM_LEN: usize = 1;
    const CALL_PTR: usize = 3;
    const CALL_LEN: usize = 4;
    const GAS_PRICE: usize = 6;
    const GAS_PTR: usize = 7;
    const STATUS: usize = 8;
    const OUTPUT_PTR: usize = 9;
    const OUTPUT_LEN: usize = 10;
    const OUTPUT_CAP: usize = 11;

    let gas_ptr = sp.read_u64(GAS_PTR) as usize;

    let mut need_to_clear = None;

    macro_rules! output {
        ($status:expr, $output:expr) => {{
            let output: Vec<u8> = $output.into();
            sp.write_u64(STATUS, $status);
            write_output(sp, &output, OUTPUT_PTR, OUTPUT_LEN, OUTPUT_CAP);
            std::mem::forget(output);
            if let Some(module) = need_to_clear {
                clear_program(module);
            }
            return;
        }};
    }

    let wasm = read_go_slice(sp, WASM_PTR, WASM_LEN);
    let args = read_go_slice(sp, CALL_PTR, CALL_LEN);
    let args_len = args.len();

    let config = PolyglotConfig::default();
    let machine = match Machine::from_polyglot_binary(&wasm, true, &config) {
        Ok(machine) => machine,
        Err(error) => output!(1, error.to_string()),
    };

    let (hash, main, internals) = machine.main_module_info();
    let module = wavm_link_program(&MemoryLeaf(hash.0));
    wavm_prep_program(module, internals, wavm_util::wavm_caller_load64(gas_ptr));
    write_args(module, args.as_ptr(), args_len, sp.read_u64(GAS_PRICE));
    need_to_clear = Some(module);

    let status = wavm_call_program(module, main, args_len);
    if wavm_read_program_depth_left(module, internals) == 0 {
        output!(1, "out of stack");
    }
    if wavm_read_program_gas_status(module, internals) != 0 {
        output!(1, "out of gas");
    }
    let gas_left = wavm_read_program_gas_left(module, internals);
    wavm_util::wavm_caller_store64(gas_ptr, gas_left);

    let output_len = read_output_len(module);
    let mut output = Vec::with_capacity(output_len);
    read_output(module, output.as_mut_ptr());
    output.set_len(output_len);
    output!(status.into(), output);
}

#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_polyglotCopy(
    sp: GoStack,
) {
    // func(dest *byte, source *byte, len uint64)
    let dest = usize::try_from(sp.read_u64(0)).expect("Go pointer didn't fit in usize");
    let src = usize::try_from(sp.read_u64(1)).expect("Go pointer didn't fit in usize") as *mut u8;
    let len = sp.read_u64(2).try_into().unwrap();

    let input = slice::from_raw_parts(src, len);
    wavm_util::write_slice(input, dest)
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

unsafe fn read_go_slice(sp: GoStack, ptr: usize, len: usize) -> Vec<u8> {
    let wasm_ptr = sp.read_u64(ptr);
    let wasm_len = sp.read_u64(len);
    go_abi::read_slice(wasm_ptr, wasm_len)
}

unsafe fn write_output(sp: GoStack, output: &Vec<u8>, outptr: usize, outlen: usize, outcap: usize) {
    sp.write_u64(outptr, output.as_ptr() as u64);
    sp.write_u64(outlen, output.len() as u64);
    sp.write_u64(outcap, output.capacity() as u64);
}
