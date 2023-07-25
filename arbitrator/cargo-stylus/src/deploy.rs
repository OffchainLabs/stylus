// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use std::str::FromStr;

use ethers::{
    signers::{LocalWallet, Signer},
    types::{Eip1559TransactionRequest, U256, H160},
};

use crate::{constants, project, tx, DeployConfig, WalletSource};

pub fn atomic_deploy_and_compile(cfg: DeployConfig) -> eyre::Result<(), String> {
    // let multicall_data = prepare_deploy_compile_multicall(&code, &contract_addr);

    // let to = hex::decode(constants::MULTICALL_ADDR).unwrap();
    // let tx = Eip1559TransactionRequest::new()
    Ok(())
}

/// Sends a signed program deployment tx to a backend provider 
/// and returns the deployed program's address.
pub fn send_deploy_program_tx(cfg: DeployConfig) -> eyre::Result<(), String> {
    let wasm_file_path = project::build_project_to_wasm()?;
    let wasm_file_bytes = project::get_compressed_wasm_bytes(&wasm_file_path)?;
    let wallet = load_wallet(&cfg.wallet)?;

    let prepare_tx = |base_fee: U256| {
        let deployment_calldata = program_deployment_calldata(&wasm_file_bytes);
        Eip1559TransactionRequest::new()
            .from(wallet.address())
            .max_fee_per_gas(base_fee)
            .data(deployment_calldata)
    };
    tx::submit_signed_tx(
        &cfg.endpoint,
        wallet,
        cfg.estimate_gas_only,
        prepare_tx,
    );
    Ok(())
}

/// Sends a signed program compilation tx to a backend provider for the specified program address.
pub fn send_compile_program_tx(cfg: DeployConfig) -> eyre::Result<(), String> {
    let wallet = load_wallet(&cfg.wallet)?;
    let program_addr = H160::zero();
    let mut compile_calldata = vec![];
    let compile_method_hash = hex::decode("2e50f32b").unwrap();
    compile_calldata.extend(compile_method_hash);
    compile_calldata.extend(hex::decode("000000000000000000000000").unwrap());
    compile_calldata.extend(program_addr.as_bytes());

    let to = hex::decode(constants::ARB_WASM_ADDRESS).unwrap();
    let to = H160::from_slice(&to);

    let prepare_tx = |base_fee: U256| {
        Eip1559TransactionRequest::new()
            .from(wallet.address())
            .to(to)
            .max_fee_per_gas(base_fee)
            .data(compile_calldata)
    };
    tx::submit_signed_tx(
        &cfg.endpoint,
        wallet,
        cfg.estimate_gas_only,
        prepare_tx,
    );
    Ok(())
}

/// Loads a wallet for signing transactions either from a private key file path.
/// or a keystore along with a keystore password file.
fn load_wallet(cfg: &WalletSource) -> eyre::Result<LocalWallet, String> {
    if let Some(priv_key_path) = &cfg.private_key_path {
        let privkey = std::fs::read_to_string(priv_key_path)
            .map_err(|e| format!("Could not read private key file {}", e))?;
        return LocalWallet::from_str(privkey.as_str())
            .map_err(|e| format!("Could not parse private key {}", e));
    }
    let keystore_password_path = cfg
        .keystore_password_path
        .as_ref()
        .ok_or("No keystore password path provided")?;
    let keystore_path = cfg
        .keystore_path
        .as_ref()
        .ok_or("No keystore path provided")?;
    let keystore_pass = std::fs::read_to_string(keystore_password_path)
        .map_err(|e| format!("Could not keystore password file {}", e))?;
    LocalWallet::decrypt_keystore(keystore_path, keystore_pass)
        .map_err(|e| format!("Could not decrypt keystore {}", e))
}

fn program_deployment_calldata(code: &[u8]) -> Vec<u8> {
    let mut code_len = [0u8; 32];
    U256::from(code.len()).to_big_endian(&mut code_len);
    let mut deploy: Vec<u8> = vec![];
    deploy.push(0x7f); // PUSH32
    deploy.extend(code_len);
    deploy.push(0x80); // DUP1
    deploy.push(0x60); // PUSH1
    deploy.push(0x2a); // 42 the prelude length
    deploy.push(0x60); // PUSH1
    deploy.push(0x00);
    deploy.push(0x39); // CODECOPY
    deploy.push(0x60); // PUSH1
    deploy.push(0x00);
    deploy.push(0xf3); // RETURN
    let prelude = hex::encode(&deploy);
    deploy.extend(code);
    deploy
}