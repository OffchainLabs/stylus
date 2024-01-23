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
    console,
    storage::{transient_load_bytes32, transient_store_bytes32},
    stylus_proc::entrypoint,
};

#[entrypoint]
fn user_main(input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    let dest = Address::from_slice(input[20..40].try_into().unwrap());

    let storage_slot = uint!(0_U256).into();
    if input[40] == 0 {
        let data;
        unsafe{data = transient_load_bytes32(storage_slot)};
        if data != B256::from(uint!(10_U256)) {
            // Transient data should have been set to 10 by transient_reenter
            // before callback
            return Err(input);
        }
        unsafe{transient_store_bytes32(storage_slot, uint!(15_U256).into())};
        return Ok(input);
    }
    console!("counter: {}", input[40]);

    unsafe{transient_store_bytes32(storage_slot, uint!(5_U256).into())}

    if let Err(result) = unsafe{RawCall::new_delegate().call(dest, input.as_slice())} {
        return Err(result);
    }

    let data;
    unsafe{data = transient_load_bytes32(storage_slot)};
    if data != B256::from(uint!(20_U256)) {
        // Transient data should have been set to 20 by transient_reenter
        // before return
        return Err(input);
    }

    Ok(input)
}
