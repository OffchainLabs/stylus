// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

extern "C" {
    fn wavm_get_caller_module() -> u32;
}

#[no_mangle]
pub unsafe extern "C" fn polyglot__read_args(ptr: usize, module: u32) {
    //write_slice_to(&ARGS[wavm_get_caller_module()], module, ptr);
}
