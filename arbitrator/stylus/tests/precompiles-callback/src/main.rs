// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![no_main]

use arbitrum::{Bytes20, Bytes32, ecrecover_callback, debug};
use hex_literal::hex;

arbitrum::arbitrum_main!(user_main);

fn user_main(_input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    // Test ecrecover
    let hash = Bytes32(hex!["58749F0B9677F513B6CF2A4E163DC7A09D61D6E4168E05B25FD11A4FFD62944C"]);
    let v = Bytes32(hex!["000000000000000000000000000000000000000000000000000000000000001B"]);
    let r = Bytes32(hex!["98A5450851BEA26F56FA19565AE4CD26C3A63A296FDB7DBA64EEA233C930FC8D"]);
    let s = Bytes32(hex!["1DF7DD93A6A3F32196C14AB7BAD12B233D908FBFB1119157536276935EB77F30"]);
    let expected_signature = Bytes20(hex!["8E95129F90F0619801A2BF29BB4A11BBF34BE1C4"]);
    let recovered_signature = ecrecover_callback(hash, v, r, s);
    if recovered_signature != expected_signature {
        debug::println(format!("ecrecover signature didn't match {expected_signature} {recovered_signature}"));
        return Err(vec![])
    }

    Ok(vec![])
}
