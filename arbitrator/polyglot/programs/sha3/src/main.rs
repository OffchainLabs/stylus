// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![no_main]

mod arbitrum;

use sha3::{Digest, Keccak256};

// TODO: make proc macro
arbitrum::arbitrum_main!(user_main);

fn user_main(preimage: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    let hash = keccak(&preimage);
    Ok(hash.as_ref().into())
}

fn keccak(preimage: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(preimage);
    hasher.finalize().into()
}
