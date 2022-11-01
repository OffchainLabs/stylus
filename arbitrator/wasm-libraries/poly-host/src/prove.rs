// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use arbutil::color;
use go_abi::GoStack;
use prover::{
    programs::{ExecProgram, PolyglotConfig},
    Machine,
};

#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_polyglotCheck(
    sp: GoStack,
) {
    println!("{}", color::red("polyglotCheck"));

    // func (wasm []byte) (status uint64, output *byte, outlen, outcap uint64)
    let wasm_ptr = sp.read_u64(0);
    let wasm_len = sp.read_u64(1);
    const STATUS: usize = 3;
    const OUTPTR: usize = 4;
    const OUTLEN: usize = 5;
    const OUTCAP: usize = 6;

    let wasm = go_abi::read_slice(wasm_ptr, wasm_len);

    let config = PolyglotConfig::default();
    let machine = Machine::from_polyglot_binary(&wasm, true, &config);
    let status = if machine.is_err() { 1 } else { 0 };
    let output = machine.map_err(|e| e.to_string()).err().unwrap_or_default();

    println!("{} {}", color::red("RUST OUT:"), color::red(&output));
    let output = output.as_bytes().to_vec();

    sp.write_u64(STATUS, status);
    write_output(sp, &output, OUTPTR, OUTLEN, OUTCAP);
    std::mem::forget(output);
}

#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_polyglotCall(
    sp: GoStack,
) {
    println!("{}", color::red("polyglotCall"));

    // func (wasm, calldata []byte, gas_price uint64, gas *uint64) (status uint64, output *byte, outlen, outcap uint64)
    const STATUS: usize = 8;
    const OUTPTR: usize = 9;
    const OUTLEN: usize = 10;
    const OUTCAP: usize = 11;

    let output = vec![];

    sp.write_u64(STATUS, 1);
    write_output(sp, &output, OUTPTR, OUTLEN, OUTCAP);
    std::mem::forget(output);
}

#[no_mangle]
pub unsafe extern "C" fn go__github_com_offchainlabs_nitro_arbos_programs_polyglotFree(
    sp: GoStack,
) {
    println!("{}", color::red("polyglotFree"));

    // func(output *byte, outlen, outcap uint64)
    let ptr = usize::try_from(sp.read_u64(0)).expect("Go pointer didn't fit in usize") as *mut u8;
    let len = sp.read_u64(1).try_into().unwrap();
    let cap = sp.read_u64(2).try_into().unwrap();

    let vec = Vec::from_raw_parts(ptr, len, cap);
    std::mem::drop(vec)
}

unsafe fn write_output(sp: GoStack, output: &Vec<u8>, outptr: usize, outlen: usize, outcap: usize) {
    sp.write_u64(outptr, output.as_ptr() as u64);
    sp.write_u64(outlen, output.len() as u64);
    sp.write_u64(outcap, output.capacity() as u64);
}
