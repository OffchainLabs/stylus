// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![no_main]
#![no_std]
extern crate alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use {
    alloc::vec::Vec,
    stylus_sdk::{
        alloy_primitives::{Address, B256, uint},
        call::RawCall,
        console,
        storage::{transient_load_bytes32, transient_store_bytes32},
        stylus_proc::entrypoint,
    },
};


#[entrypoint]
fn user_main(input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    let dest = Address::from_slice(input[20..40].try_into().unwrap());

    let storage_slot = uint!(0_U256).into();
    if input[40] == 0 {
        let data;
        unsafe{data = transient_load_bytes32(storage_slot)}
        if data != B256::from(uint!(5_U256)) {
            // Transient data should have been left at 5 by transient_enter
            // before calling transient_reenter and left that at that value
            return Err(input)
        }
        unsafe{transient_store_bytes32(storage_slot, uint!(15_U256).into())};
        return Ok(input);
    }
    console!("counter: {}", input[40]);

    unsafe{transient_store_bytes32(storage_slot, uint!(5_U256).into())}

    if let Err(result) = unsafe{RawCall::new().call(dest, input.as_slice())} {
        return Err(result);
    }

    let data;
    unsafe{data = transient_load_bytes32(storage_slot)};
    if data != B256::from(uint!(15_U256)) {
        // Transient data should have been set to 15 by transient_enter during reentry
        // and call to transient_reenter should have left it at that at that value
        return Err(input);
    }

    Ok(input)

}
