#![allow(clippy::let_unit_value)]

use crate::test::{run_native, test_configs, TestInstance};
use alloy_primitives::{Address, U256};
use alloy_sol_types::{sol_data, SolType};
use eyre::Result;

const FILENAME: &str = "tests/router-entry/target/wasm32-unknown-unknown/release/router-entry.wasm";

/**
 * tasks:
 *   - generate selector arms automatically
 *   - create route & router structs and define any traits needed
 *   - limit arg encoding to primitive types, will need to handle structs, etc
 *   -  
 */

fn run_it(input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    let (compile, config, ink) = test_configs();
    let mut native = TestInstance::new_linked(FILENAME, &compile, config).unwrap();
    let output = run_native(&mut native, &input, ink).unwrap();
    Ok(output)
}

#[test]
fn test_encode() -> Result<()> {
    // sol_data: Address, Array, Bool, ByteCount, Bytes, FixedArray, FixedBytes, Int, IntBitCount, String, Uint
    // sol_data::Address::from([0x22u8; 20])
    //
    type Params = (sol_data::Address, sol_data::Address);
    let addresses = (Address::from([0xafu8; 20]), Address::from([0x22u8; 20]));
    // let encoded = encode_params::<(sol_data::Address, sol_data::Address)>(&addresses);
    let encoded = Params::encode_params(&addresses);
    let hex_encoded = Params::hex_encode_params(&addresses);

    println!("encoded: {:02x?}", encoded);
    println!("hex: {}", hex_encoded);

    Ok(())
}

#[test]
fn test_router() -> Result<()> {
    let transfer_selector: u32 = 0x8a4068dd;
    let balance_selector_snake: u32 = 0x4668f7f4;
    let balance_selector_camel: u32 = 0x722713f7;
    let return_one_selector: u32 = 0x00000001;

    let address1 = Address::from([0x11u8; 20]);
    let _uint = U256::from_be_bytes::<32>([0x11u8; 32]);
    let uint2 = U256::from(1_000_000);

    type TransferParams = (sol_data::Address, sol_data::Uint<256>);
    type BalanceOfParams = sol_data::Address;
    type ReturnOneParams = ();

    let transfer_args = (address1, uint2);
    let balance_args = address1;
    let return_one_args = ();

    let transfer_args_encoded = TransferParams::encode_single(&transfer_args);
    let balance_args_encoded = BalanceOfParams::encode_single(&balance_args);
    let return_one_args_encoded = ReturnOneParams::encode_single(&return_one_args);

    let transfer_calldata = [
        transfer_selector.to_be_bytes().as_slice(),
        transfer_args_encoded.as_slice(),
    ]
    .concat();

    let balance_calldata_snake = [
        balance_selector_snake.to_be_bytes().as_slice(),
        balance_args_encoded.as_slice(),
    ]
    .concat();

    let balance_calldata_camel = [
        balance_selector_camel.to_be_bytes().as_slice(),
        balance_args_encoded.as_slice(),
    ]
    .concat();

    let return_one_calldata = [
        return_one_selector.to_be_bytes().as_slice(),
        return_one_args_encoded.as_slice(),
    ]
    .concat();

    let _output = run_it(transfer_calldata).unwrap();
    let _output = run_it(balance_calldata_camel).unwrap();
    let _output = run_it(balance_calldata_snake).unwrap();
    let _output = run_it(return_one_calldata).unwrap();

    Ok(())
}
