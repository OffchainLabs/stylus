// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![no_main]
#![no_std]
extern crate alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use alloc::vec::Vec;

use stylus_sdk::{
    alloy_primitives::{Address, B256, uint},
    call::RawCall,
    storage::{transient_load_bytes32, transient_store_bytes32},
    stylus_proc::entrypoint,
};

#[entrypoint]
fn user_main(input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    let dest = Address::from_slice(input[0..20].try_into().unwrap());

    if input[40] == 0 {
        // Should never be reached
        return Err(input);
    }

    let storage_slot = uint!(0_U256).into();
    let data;
    unsafe{data = transient_load_bytes32(storage_slot)};
    if data != B256::from(uint!(5_U256)) {
        // Transient data should have been set to 5 by transient_enter
        // before call
        return Err(input);
    }

    unsafe{transient_store_bytes32(storage_slot, uint!(10_U256).into())};

    let mut input = input;
    input[40] = 0;
    if let Err(result) = unsafe{RawCall::new().call(dest, input.as_slice())} {
        return Err(result);
    }



    let data2;
    unsafe{data2 = transient_load_bytes32(storage_slot)};
    if data2 != B256::from(uint!(15_U256)) {
        // Transient data should have been set to 15 by transient_enter
        // before return
        return Err(input);
    }

    unsafe{transient_store_bytes32(storage_slot, uint!(20_U256).into())};

    Ok(input)
}
