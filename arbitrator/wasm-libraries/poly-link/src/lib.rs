// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use arbutil::color;
use go_abi::GoStack;
use prover::{
    programs::{ExecOutcome, ExecProgram, PolyglotConfig},
    utils::Bytes32,
    Machine,
};

extern "C" {
    fn wavm_link_module(hash: Bytes32);
}

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
    const STATUS: usize = 8;
    const OUTPTR: usize = 9;
    const OUTLEN: usize = 10;
    const OUTCAP: usize = 11;

    macro_rules! output {
        ($status:expr, $output:expr) => {{
            let output: Vec<u8> = $output.into();
            sp.write_u64(STATUS, $status);
            write_output(sp, &output, OUTPTR, OUTLEN, OUTCAP);
            std::mem::forget(output);
            return;
        }};
    }

    let wasm = read_go_slice(sp, WASM_PTR, WASM_LEN);
    let data = read_go_slice(sp, CALL_PTR, CALL_LEN);

    color::redln("from binary");

    let config = PolyglotConfig::default();
    let machine = match Machine::from_polyglot_binary(&wasm, true, &config) {
        Ok(machine) => machine,
        Err(error) => output!(1, error.to_string()),
    };

    let hash = machine.get_main_module_hash();
    color::blueln(format!("Linking Module {hash}"));
    wavm_link_module(hash);
    color::blueln(format!("Linked Module {hash}"));

    /*let outcome = match machine.run_main(data) {
        Ok(outcome) => outcome,
        Err(error) => output!(1, error.to_string()),
    };

    println!("{} {outcome}", color::red("post main"));

    use ExecOutcome::*;
    match outcome {
        Success(data) => output!(0, data),
        Revert(data) => output!(1, data),
        Failure(error) => output!(1, error.to_string()),
        OutOfGas => output!(1, "out of gas"),
        OutOfStack => output!(1, "stack overflow"),
        DivergingFailure => output!(2, "diverging failure"),
    }*/
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
