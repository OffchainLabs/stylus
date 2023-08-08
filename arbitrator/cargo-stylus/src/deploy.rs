use std::path::PathBuf;
// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use std::str::FromStr;

use ethers::types::{Eip1559TransactionRequest, H160, U256};
use ethers::utils::get_contract_address;
use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
};

use crate::{constants, multicall, project, tx, DeployConfig, DeployMode, WalletSource};

/// Performs one of three different modes for a Stylus program:
/// DeployOnly: Sends a signed tx to deploy a Stylus program to a new address.
/// CompileOnly: Sends a signed tx to compile a Stylus program at a specified address.
/// DeployAndCompile (default): Sends a signed, multicall tx to both deploy and compile a Stylus program atomically.
pub async fn deploy(cfg: DeployConfig) -> eyre::Result<(), String> {
    let wasm_file_path: PathBuf = match &cfg.wasm_file_path {
        Some(path) => PathBuf::from_str(&path).unwrap(),
        None => project::build_project_to_wasm()?,
    };
    let wasm_file_bytes = project::get_compressed_wasm_bytes(&wasm_file_path)?;
    let wallet = load_wallet(&cfg.wallet)?;

    let provider = Provider::<Http>::try_from(&cfg.endpoint)
        .map_err(|e| format!("could not initialize provider from http {}", e))?;
    let chain_id = provider
        .get_chainid()
        .await
        .map_err(|e| format!("could not get chain id {}", e))?
        .as_u64();
    let client = SignerMiddleware::new(provider, wallet.clone().with_chain_id(chain_id));

    let addr = wallet.address();
    let nonce = client
        .get_transaction_count(addr, None)
        .await
        .map_err(|e| format!("could not get nonce {} {}", addr, e))?;

    let mut tx_request = prepare_tx_request(&cfg, &wasm_file_bytes, &wallet, nonce)?;

    tx::submit_signed_tx(client, cfg.estimate_gas_only, &mut tx_request).await
}

/// Prepares a tx request for the given deploy config. For deploying a program only, it
/// prepares a contract creation transaction. For compiling only, it prepares a compilation tx,
/// and for both deploying and atomically compiling, prepares a multicall tx.
fn prepare_tx_request(
    cfg: &DeployConfig,
    wasm_file_bytes: &[u8],
    wallet: &LocalWallet,
    nonce: U256,
) -> eyre::Result<Eip1559TransactionRequest, String> {
    match cfg.mode {
        Some(DeployMode::DeployOnly) => {
            let program_addr = get_contract_address(wallet.address(), nonce);
            println!("Deploying program to address {program_addr:#032x}");
            let deployment_calldata = program_deployment_calldata(wasm_file_bytes);
            Ok(Eip1559TransactionRequest::new()
                .from(wallet.address())
                .data(deployment_calldata))
        }
        Some(DeployMode::CompileOnly) => {
            let program_addr = cfg
                .compile_program_addr
                .ok_or("No --compile-program-addr provided")?;
            let mut compile_calldata = vec![];
            let compile_method_hash = hex::decode(constants::ARBWASM_COMPILE_METHOD_HASH).unwrap();
            compile_calldata.extend(compile_method_hash);
            compile_calldata.extend(hex::decode("000000000000000000000000").unwrap());
            compile_calldata.extend(program_addr.as_bytes());

            let to = hex::decode(constants::ARB_WASM_ADDRESS).unwrap();
            let to = H160::from_slice(&to);

            Ok(Eip1559TransactionRequest::new()
                .from(wallet.address())
                .to(to)
                .data(compile_calldata))
        }
        // Default mode is to deploy and compile atomically via a multicall Stylus program.
        None => {
            let program_addr = get_contract_address(wallet.address(), nonce);
            println!("Deploying program to address {program_addr:#032x}");
            let multicall_data =
                multicall::prepare_deploy_compile_multicall(wasm_file_bytes, &program_addr);

            let to = hex::decode(constants::MULTICALL_ADDR).unwrap();
            let to = H160::from_slice(&to);
            Ok(Eip1559TransactionRequest::new()
                .to(to)
                .from(wallet.address())
                .data(multicall_data))
        }
    }
}

/// Loads a wallet for signing transactions either from a private key file path.
/// or a keystore along with a keystore password file.
fn load_wallet(cfg: &WalletSource) -> eyre::Result<LocalWallet, String> {
    if let Some(priv_key_path) = &cfg.private_key_path {
        let privkey = std::fs::read_to_string(priv_key_path)
            .map_err(|e| format!("could not read private key file {}", e))?;
        return LocalWallet::from_str(privkey.as_str())
            .map_err(|e| format!("could not parse private key {}", e));
    }
    let keystore_password_path = cfg
        .keystore_password_path
        .as_ref()
        .ok_or("no keystore password path provided")?;
    let keystore_path = cfg
        .keystore_path
        .as_ref()
        .ok_or("no keystore path provided")?;
    let keystore_pass = std::fs::read_to_string(keystore_password_path)
        .map_err(|e| format!("could not keystore password file {}", e))?;
    LocalWallet::decrypt_keystore(keystore_path, keystore_pass)
        .map_err(|e| format!("could not decrypt keystore {}", e))
}

/// Prepares an EVM bytecode prelude for contract creation.
pub fn program_deployment_calldata(code: &[u8]) -> Vec<u8> {
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
    deploy.extend(code);
    deploy
}
