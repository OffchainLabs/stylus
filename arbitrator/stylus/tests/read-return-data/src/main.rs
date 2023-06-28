// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![no_main]

use arbitrum::{call::Call, debug};

arbitrum::arbitrum_main!(user_main);

fn user_main(input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    let offset = u64::from_be_bytes(input[..8].try_into().unwrap());
    let size = u64::from_be_bytes(input[8..16].try_into().unwrap());

    debug::println(format!("checking return data subset: {offset} {size}"));
    // Call identity precompile to test return data
    let call_data: [u8; 4] = [0, 1, 2, 3];
    let identity_precompile: u32 = 0x4;
    let call_return_data = Call::new().
            subset(offset, size).
            call(identity_precompile.into(), &call_data)?;
    for (index, item) in call_return_data.iter().enumerate() {
        if *item != call_data[offset as usize + index] {
            debug::println(
                format!(
                    "returned data incorrect: out[{index}] {item} != data[{offset} + {index}] {}",
                    call_data[offset as usize + index],
                ),
            );
            panic!("invalid data");
        }
    }

    Ok(call_return_data)
}
