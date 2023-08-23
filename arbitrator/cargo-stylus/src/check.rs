// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use bytesize::ByteSize;

use arbutil::Color;

use ethers::{
    providers::JsonRpcClient,
    types::{transaction::eip2718::TypedTransaction, Address},
};

use crate::{
    constants::{ARB_WASM_ADDRESS, MAX_PROGRAM_SIZE},
    deploy::activation_calldata,
};

use ethers::types::Eip1559TransactionRequest;
use ethers::{
    core::types::spoof,
    providers::{Provider, RawCall},
};

/// Defines the stylus checks that occur during the compilation of a WASM program
/// into a module. Checks can be disabled during the compilation process for debugging purposes.
#[derive(PartialEq)]
pub enum StylusCheck {
    CompressedSize,
    // TODO: Adding more checks here would require being able to toggle
    // compiler middlewares in the compile config store() method.
}

impl TryFrom<&str> for StylusCheck {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "compressed-size" => Ok(StylusCheck::CompressedSize),
            _ => Err(format!("invalid Stylus middleware check: {}", value,)),
        }
    }
}

/// Runs a series of checks on the WASM program to ensure it is valid for compilation
/// and code size before being deployed and activated onchain. An optional list of checks
/// to disable can be specified.
pub fn run_checks(
    wasm_file_bytes: &[u8],
    deploy_ready_code: &[u8],
    disabled: Vec<StylusCheck>,
) -> eyre::Result<(), String> {
    let compressed_size = ByteSize::b(deploy_ready_code.len() as u64);
    let check_compressed_size = disabled.contains(&StylusCheck::CompressedSize);

    if check_compressed_size && compressed_size > MAX_PROGRAM_SIZE {
        return Err(format!(
            "Brotli-compressed WASM size {} is bigger than program size limit: {}",
            compressed_size.to_string().red(),
            MAX_PROGRAM_SIZE,
        ));
    }
    //check_can_activate(client, expected_program_address, compressed_wasm)
    Ok(())
}

/// Checks if a program can be successfully activated onchain before it is deployed
/// by using an eth_call override that injects the program's code at a specified address.
/// This allows for verifying an activation call is correct and will succeed if sent
/// as a transaction with the appropriate gas.
pub async fn check_can_activate<T>(
    client: Provider<T>,
    expected_program_address: &Address,
    compressed_wasm: Vec<u8>,
) -> eyre::Result<(), String>
where
    T: JsonRpcClient + Sync + Send + std::fmt::Debug,
{
    let calldata = activation_calldata(expected_program_address);
    let to = hex::decode(ARB_WASM_ADDRESS).unwrap();
    let to = Address::from_slice(&to);

    let tx_request = Eip1559TransactionRequest::new().to(to).data(calldata);
    let tx = TypedTransaction::Eip1559(tx_request);

    // Spoof the state as if the program already exists at the specified address
    // using an eth_call override.
    let state = spoof::code(
        Address::from_slice(expected_program_address.as_bytes()),
        compressed_wasm.into(),
    );
    let response = client
        .call_raw(&tx)
        .state(&state)
        .await
        .map_err(|e| format!("program predeployment check failed: {e}"))?;

    println!("Got response: {}", hex::encode(&response));
    Ok(())
}
