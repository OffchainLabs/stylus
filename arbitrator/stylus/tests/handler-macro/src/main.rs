// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![no_main]

use stylus_sdk::{
    alloy_primitives::{Address, B256, U256},
    alloy_sol_types::{sol_data, SolType},
    debug, load_bytes32, store_bytes32,
    stylus_proc::{handler, router},
};

stylus_sdk::entrypoint!(user_main);

// TODO: Generate code within handler macro to expand and decode input: Vec<u8> passed to handler from router

// #[handler]
// fn empty() {
//     debug::println("empty fn");
// }

trait Handler {
    const SIGNATURE: &'static str;
}

#[handler]
fn balance_of(account: sol_data::Address) -> (sol_data::Uint<256>, sol_data::Uint<256>) {
    debug::println(format!("args; account: {}", account.into_word()));

    (U256::from(2_000_000), U256::from(2_000_000))
}

#[handler]
fn transfer(recipient: sol_data::Address, amount: sol_data::Uint<256>) {
    debug::println(format!(
        "args; recipient: {}, amount: {}",
        recipient, amount
    ));
}

fn user_main(input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    router! {
      "balance_of" => balance_of,
      "transfer" => transfer,
    };
}
