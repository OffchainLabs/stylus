// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![no_main]
#![no_std]
extern crate alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use alloc::vec::Vec;

use stylus_sdk::{
    alloy_primitives::Address,
    call::RawCall,
    console,
    stylus_proc::entrypoint,
};

#[entrypoint]
fn user_main(input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    let dest = Address::from_slice(input[20..40].try_into().unwrap());

    if input[40] == 0 {
        return Ok(input)
    }
    console!("counter: {}", input[40]);

    let mut input = input.to_vec();
    input[40] -= 1;
    RawCall::new().call(dest, input.as_slice())
}
