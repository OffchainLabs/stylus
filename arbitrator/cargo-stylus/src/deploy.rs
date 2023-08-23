use std::io::{BufRead, BufReader};
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

use crate::{constants, project, tx, DeployConfig, DeployMode};

/// Performs one of three different modes for a Stylus program:
/// DeployOnly: Sends a signed tx to deploy a Stylus program to a new address.
/// ActivateOnly: Sends a signed tx to activate a Stylus program at a specified address.
/// DeployAndActivate (default): Sends both transactions above.
pub async fn deploy(cfg: DeployConfig) -> eyre::Result<(), String> {
    let wallet = load_wallet(&cfg)?;

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

    let expected_program_addr = get_contract_address(wallet.address(), nonce);

    let (deploy, activate) = match cfg.mode {
        Some(DeployMode::DeployOnly) => (true, false),
        Some(DeployMode::ActivateOnly) => (false, true),
        // Default mode is to deploy and activate
        None => {
            if cfg.estimate_gas_only && cfg.activate_program_address.is_none() {
                // cannot activate if not really deploying
                (true, false)
            } else {
                (true, true)
            }
        }
    };

    if deploy {
        let wasm_file_path: PathBuf = match &cfg.wasm_file_path {
            Some(path) => PathBuf::from_str(&path).unwrap(),
            None => project::build_project_to_wasm()?,
        };
        let (_, deploy_ready_code) = project::get_compressed_wasm_bytes(&wasm_file_path)?;
        println!("Deploying program to address {expected_program_addr:#032x}");
        let deployment_calldata = program_deployment_calldata(&deploy_ready_code);
        let mut tx_request = Eip1559TransactionRequest::new()
            .from(wallet.address())
            .data(deployment_calldata);
        tx::submit_signed_tx(&client, cfg.estimate_gas_only, &mut tx_request).await?;
    }
    if activate {
        let program_addr = cfg
            .activate_program_address
            .unwrap_or(expected_program_addr);
        println!("Activating program at address {program_addr:#032x}");
        let activate_calldata = activation_calldata(&program_addr);

        let to = hex::decode(constants::ARB_WASM_ADDRESS).unwrap();
        let to = H160::from_slice(&to);

        let mut tx_request = Eip1559TransactionRequest::new()
            .from(wallet.address())
            .to(to)
            .data(activate_calldata);
        tx::submit_signed_tx(&client, cfg.estimate_gas_only, &mut tx_request).await?;
    }
    Ok(())
}

/// Loads a wallet for signing transactions either from a private key file path.
/// or a keystore along with a keystore password file.
fn load_wallet(cfg: &DeployConfig) -> eyre::Result<LocalWallet, String> {
    if cfg.private_key_path.is_some()
        && (cfg.keystore_opts.keystore_password_path.is_some()
            && cfg.keystore_opts.keystore_path.is_some())
    {
        return Err("must provide either privkey path or keystore options exclusively".to_string());
    }

    if let Some(priv_key_path) = &cfg.private_key_path {
        let privkey = read_secret_from_file(&priv_key_path)?;
        return LocalWallet::from_str(&privkey)
            .map_err(|e| format!("could not parse private key {}", e));
    }
    let keystore_password_path = cfg
        .keystore_opts
        .keystore_password_path
        .as_ref()
        .ok_or("no keystore password path provided")?;
    let keystore_pass = read_secret_from_file(&keystore_password_path)?;
    let keystore_path = cfg
        .keystore_opts
        .keystore_path
        .as_ref()
        .ok_or("no keystore path provided")?;
    LocalWallet::decrypt_keystore(keystore_path, keystore_pass)
        .map_err(|e| format!("could not decrypt keystore {}", e))
}

fn read_secret_from_file(fpath: &str) -> Result<String, String> {
    let f =
        std::fs::File::open(fpath).map_err(|e| format!("could not read secret from file {}", e))?;
    let mut buf_reader = BufReader::new(f);
    let mut secret = String::new();
    buf_reader
        .read_line(&mut secret)
        .map_err(|e| format!("could not read secret from file {}", e))?;
    Ok(secret.trim().to_string())
}

pub fn activation_calldata(program_addr: &H160) -> Vec<u8> {
    let mut activate_calldata = vec![];
    let activate_method_hash = hex::decode(constants::ARBWASM_ACTIVATE_METHOD_HASH).unwrap();
    activate_calldata.extend(activate_method_hash);
    let mut extension = [0u8; 32];
    // Next, we add the address to the last 20 bytes of extension
    extension[12..32].copy_from_slice(program_addr.as_bytes());
    activate_calldata.extend(extension);
    activate_calldata
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
