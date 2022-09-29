// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![allow(dead_code)]

extern "C" {
    fn read_args(data: *const u8);
    fn return_data(status: usize, len: usize, data: *const u8) -> !;
}

pub (crate) fn args(len: usize) -> Vec<u8> {
    let mut input = Vec::with_capacity(len);
    unsafe {
        read_args(input.as_ptr());
        input.set_len(len);
    }
    input
}

fn exit_with_status(data: &[u8], status: usize) -> ! {
    unsafe {
        let len = data.len();
        let out = data.as_ptr();
        std::mem::forget(data); // leak the data
        return_data(status, len, out);
    }
}

pub (crate) fn exit_success(data: &[u8]) -> ! {
    exit_with_status(data, 0)
}

pub (crate) fn exit_failure(data: &[u8]) -> ! {
    exit_with_status(data, 1)
}

// TODO: make this a procedural macro
macro_rules! arbitrum_main {
    ($name:expr) => {
        #[no_mangle]
        pub extern "C" fn arbitrum_main(len: usize) {
            let input = arbitrum::args(len);
            let out = $name(input);
            match out {
                Ok(out) => arbitrum::exit_success(&out),
                Err(out) => arbitrum::exit_failure(&out),
            }
        }
    };
}

pub (crate) use arbitrum_main;
