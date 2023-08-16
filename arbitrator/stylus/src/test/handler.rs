#![allow(clippy::let_unit_value)]

use std::{env::args, ops::Add};

use crate::test::{run_native, test_configs, TestInstance};
use alloy_primitives::{Address, U256};
use alloy_sol_types::{encode, sol_data as abi, token::TokenSeq, Encodable, SolType};
use eyre::Result;

const FILENAME: &str =
    "tests/handler-macro/target/wasm32-unknown-unknown/release/handler-macro.wasm";

fn run_it(input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    let (compile, config, ink) = test_configs();
    let mut native = TestInstance::new_linked(FILENAME, &compile, config).unwrap();
    let output = run_native(&mut native, &input, ink).unwrap();
    Ok(output)
}

trait Sendable {
    fn prepend_selector(&self, selector: u32) -> Vec<u8>;
}

impl Sendable for Vec<u8> {
    fn prepend_selector(&self, selector: u32) -> Vec<u8> {
        [selector.to_be_bytes().as_slice(), self.as_slice()].concat()
    }
}

#[test]
fn balance_of() -> Result<()> {
    let balance_selector: u32 = 0x8a4068dd;
    let address22 = Address::from([0x22u8; 20]);

    type BalanceParams = abi::Address;
    type BalanceReturn = (abi::Uint<256>, abi::Uint<256>);
    let balance_args = address22;
    let args_encoded = BalanceParams::encode_single(&balance_args);

    let balance_msg = args_encoded.prepend_selector(balance_selector);
    let output = run_it(balance_msg).unwrap();
    let decoded_output = <BalanceReturn as SolType>::decode(&output, true);
    println!("{:?}", decoded_output);

    Ok(())
}

#[test]
fn test_transfer() -> Result<()> {
    let transfer_selector: u32 = 0x8a4068dd;
    let address1 = Address::from([0x11u8; 20]);
    let amount = U256::from(1_000_000);

    type TransferParams = (abi::Address, abi::Uint<256>);
    let transfer_args = (address1, amount);
    let args_encoded = TransferParams::encode(&transfer_args);

    let transfer_message = args_encoded.prepend_selector(transfer_selector);
    let _output = run_it(transfer_message).unwrap();

    Ok(())
}

#[test]
fn test_router() -> Result<()> {
    let _output = run_it(vec![]).unwrap();

    Ok(())
}
