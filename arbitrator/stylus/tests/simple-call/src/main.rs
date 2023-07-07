// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![no_main]

use arbitrum::{contract::Call, debug};

arbitrum::arbitrum_main!(user_main);

fn user_main(_: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    let call_data: [u8; 4] = [0, 1, 2, 3];
    let identity_precompile: u32 = 0x4;

    let return_data = Call::new().call(identity_precompile.into(), &call_data)?;
    if return_data != call_data {
        debug::println(
            format!("call_data: {call_data:#?}, unexpected return data: {return_data:#?}"),
        );
        panic!("invalid data");
    }
    let return_data = Call::new().call(identity_precompile.into(), &call_data)?;
    if return_data != call_data {
        debug::println(
            format!("call_data: {call_data:#?}, unexpected return data: {return_data:#?}"),
        );
        panic!("invalid data");
    }

    return Ok(return_data)
}
