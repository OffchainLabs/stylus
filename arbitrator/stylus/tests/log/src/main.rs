// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![no_main]

use stylus_sdk::{alloy_primitives::B256, evm};

stylus_sdk::entrypoint!(user_main);

fn user_main(input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    let num_topics = input[0];
    let mut input = &input[1..];

    let mut topics = vec![];
    for _ in 0..num_topics {
        topics.push(B256::try_from(&input[..32]).unwrap());
        input = &input[32..];
    }
    evm::log(&topics, input).unwrap();
    Ok(vec![])
}
