// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![no_main]

stylus_sdk::entrypoint!(user_main);

fn user_main(input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    Ok(input)
}
