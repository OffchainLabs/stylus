use std::convert::TryFrom;
use std::env::current_dir;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;

use brotli2::read::BrotliEncoder;
use bytes::buf::Reader;
use bytes::{Buf, Bytes};

use ethers::abi::Address;
use ethers::prelude::{
    ContractDeployer, ContractDeploymentTx, ContractFactory, MiddlewareBuilder, U256,
};
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Eip1559TransactionRequest, H160};
use ethers::utils::get_contract_address;
use ethers::{
    core::{types::TransactionRequest, utils::Anvil},
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
};

use arbutil::Color;
use eyre::bail;

use crate::{constants, DeployConfig};

pub async fn deploy_and_compile_onchain(cfg: &DeployConfig) -> eyre::Result<()> {
    let cwd: PathBuf = current_dir().unwrap();

    // TODO: Configure debug or release via flags.
    // TODO: Capture errors from this command.
    Command::new("cargo")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .arg("build")
        .arg("--release")
        .arg("--target=wasm32-unknown-unknown")
        .output()
        .expect("Failed to execute cargo build");

    let wasm_path = cwd
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{}.wasm", "echo"));

    println!("Reading compiled WASM at {}", wasm_path.display().yellow());

    let wasm_file_bytes =
        std::fs::read(&wasm_path).expect("Could not read WASM file at target path");
    println!(
        "Reading file {:?}, length in bytes {}",
        wasm_path.as_os_str(),
        wasm_file_bytes.len()
    );

    let wbytes: Reader<&[u8]> = wasm_file_bytes.reader();

    let mut compressor = BrotliEncoder::new(wbytes, constants::BROTLI_COMPRESSION_LEVEL);
    let mut compressed_bytes = vec![];
    compressor.read_to_end(&mut compressed_bytes).unwrap();

    // TODO: Add the compression and compilation checks in here. Reuse functions from check.
    println!(
        "Compressed WASM size: {} bytes",
        compressed_bytes.len().to_string().yellow()
    );
    let mut code = hex::decode(constants::EOF_PREFIX).unwrap();
    code.extend(compressed_bytes);
    println!("Compressed WASM with prefix len: {}", code.len(),);

    // Next, we prepend with the EOF bytes and prepare a compilation tx onchain. Uses ethers
    // to prepare the tx and send it over onchain to an endpoint. Will prepare a multicall data
    // tx to send to a multicall.rs rust program.
    let wallet = if let Some(priv_key_path) = &cfg.wallet.private_key_path {
        let privkey =
            std::fs::read_to_string(priv_key_path).expect("Could not read private key file");
        LocalWallet::from_str(privkey.as_str()).expect("Could not parse privkey")
    } else {
        let wallet = cfg.wallet.clone();
        let keystore_password_path = wallet
            .keystore_password_path
            .expect("No keystore password file provided");
        let keystore_path = wallet.keystore_path.expect("No keystore path provided");
        let keystore_pass = std::fs::read_to_string(keystore_password_path)
            .expect("Could not keystore password file");
        LocalWallet::decrypt_keystore(keystore_path, keystore_pass)
            .expect("Could not decrypt keystore")
    };
    submit_multicall(&cfg.endpoint, wallet, &code).await
}

fn contract_init_code(code: &[u8]) -> Vec<u8> {
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
    println!("Got prelude={prelude}");
    let first_few: [u8; 12] = code[..12].try_into().unwrap();
    let first_few = hex::encode(first_few);
    println!("First 12 bytes={first_few}");
    deploy.extend(code);
    deploy
}

fn prepare_deploy_compile_multicall(compressed_wasm: &[u8], expected_address: &H160) -> Vec<u8> {
    let code = contract_init_code(compressed_wasm);
    let mut multicall_args = args_for_multicall(MulticallArg::Call, H160::zero(), None, code);
    // let arbwasm_address = hex::decode(constants::ARB_WASM_ADDRESS).unwrap();
    // let mut compile_calldata = vec![];
    // let compile_method_hash = hex::decode("2e50f32b").unwrap();
    // compile_calldata.extend(compile_method_hash);
    // compile_calldata.extend(hex::decode("000000000000000000000000").unwrap());
    // compile_calldata.extend(expected_address.as_bytes());
    // multicall_append(
    //     &mut multicall_args,
    //     MulticallArg::Call,
    //     H160::from_slice(&arbwasm_address),
    //     compile_calldata,
    // );
    multicall_args
}

#[derive(Clone)]
enum MulticallArg {
    Call,
    DelegateCall,
    StaticCall,
}

impl From<MulticallArg> for u8 {
    fn from(value: MulticallArg) -> Self {
        match value {
            MulticallArg::Call => 0x00,
            MulticallArg::DelegateCall => 0x01,
            MulticallArg::StaticCall => 0x02,
        }
    }
}

fn args_for_multicall(
    opcode: MulticallArg,
    address: H160,
    value: Option<U256>,
    calldata: Vec<u8>,
) -> Vec<u8> {
    let mut args = vec![0x01];
    let mut length: u32 = 21 + calldata.len() as u32;
    if matches!(opcode, MulticallArg::Call) {
        length += 32;
    }
    args.extend(length.to_be_bytes());
    args.push(opcode.clone().into());

    if matches!(opcode, MulticallArg::Call) {
        let mut val = [0u8; 32];
        value.unwrap_or(U256::zero()).to_big_endian(&mut val);
        args.extend(val);
    }
    args.extend(address.as_bytes());
    args.extend(calldata);
    println!("Got args as {}", hex::encode(&args));
    args
}

fn multicall_append(calls: &mut Vec<u8>, opcode: MulticallArg, address: H160, inner: Vec<u8>) {
    calls[0] += 1; // add another call
    let args = args_for_multicall(opcode, address, None, inner);
    calls.extend(args[1..].iter().cloned());
}

async fn submit_multicall(endpoint: &str, wallet: LocalWallet, code: &[u8]) -> eyre::Result<()> {
    let provider = Provider::<Http>::try_from(endpoint)?;
    let chain_id = provider.get_chainid().await?.as_u64();
    let addr = wallet.address();
    let client = SignerMiddleware::new(provider, wallet.with_chain_id(chain_id));

    let nonce = client.get_transaction_count(addr, None).await?;
    let block_num = client.get_block_number().await?;
    let block = client.get_block(block_num).await?;
    if block.is_none() {
        bail!("No latest block found");
    }
    // TODO: Check if base fee exists.
    let base_fee = block.unwrap().base_fee_per_gas.unwrap();
    let contract_addr = get_contract_address(addr, nonce);
    let multicall_data = prepare_deploy_compile_multicall(&code, &contract_addr);

    let to = hex::decode(constants::MULTICALL_ADDR).unwrap();
    let tx = Eip1559TransactionRequest::new()
        .from(addr)
        .to(H160::from_slice(&to))
        .max_priority_fee_per_gas(base_fee)
        .data(multicall_data);
    let tx = TypedTransaction::Eip1559(tx);

    //let estimated = client.estimate_gas(&tx, None).await?;
    //println!("{estimated} estimated gas");
    println!("Sending program creation + compilation tx");
    let pending_tx = client.send_transaction(tx, None).await?;

    let receipt = pending_tx
        .await?
        .ok_or_else(|| eyre::format_err!("Tx dropped from mempool"))?;

    let tx = client.get_transaction(receipt.transaction_hash).await?;

    println!("Tx receipt: {}", serde_json::to_string(&receipt)?);
    println!("Deployed and compiled program onchain at once {contract_addr}");
    Ok(())
}

async fn submit_signed_tx(endpoint: &str, wallet: LocalWallet, code: &[u8]) -> eyre::Result<()> {
    let provider = Provider::<Http>::try_from(endpoint)?;
    let chain_id = provider.get_chainid().await?.as_u64();
    let addr = wallet.address();
    let client = SignerMiddleware::new(provider, wallet.with_chain_id(chain_id));

    let nonce = client.get_transaction_count(addr, None).await?;
    let block_num = client.get_block_number().await?;
    let block = client.get_block(block_num).await?;
    if block.is_none() {
        bail!("No latest block found");
    }
    // TODO: Check if base fee exists.
    let base_fee = block.unwrap().base_fee_per_gas.unwrap();

    // Deploy contract init code.
    let init_code = contract_init_code(&code);

    // let to = hex::decode(constants::ARB_WASM_ADDRESS).unwrap();
    // let to = H160::from_slice(&to);
    let tx = Eip1559TransactionRequest::new()
        .from(addr)
        .max_priority_fee_per_gas(base_fee)
        .data(init_code);
    let tx = TypedTransaction::Eip1559(tx);

    //let estimated = client.estimate_gas(&tx, None).await?;
    //println!("{estimated} estimated gas");

    // Get base fee, estimate gas.
    // Create a new contract creation tx.
    // Send the tx and create address from to and nonce.
    let contract_addr = get_contract_address(addr, nonce);

    println!("Sending program creation tx");
    let pending_tx = client.send_transaction(tx, None).await?;

    let receipt = pending_tx
        .await?
        .ok_or_else(|| eyre::format_err!("Tx dropped from mempool"))?;

    let tx = client.get_transaction(receipt.transaction_hash).await?;

    println!("Tx receipt: {}", serde_json::to_string(&receipt)?);
    println!("Created contract {contract_addr}");

    let nonce = client.get_transaction_count(addr, None).await?;
    let block_num = client.get_block_number().await?;
    let block = client.get_block(block_num).await?;
    if block.is_none() {
        bail!("No latest block found");
    }
    // TODO: Check if base fee exists.
    let base_fee = block.unwrap().base_fee_per_gas.unwrap();

    let mut compile_calldata = vec![];
    let compile_method_hash = hex::decode("2e50f32b").unwrap();
    compile_calldata.extend(compile_method_hash);
    compile_calldata.extend(hex::decode("000000000000000000000000").unwrap());
    compile_calldata.extend(contract_addr.as_bytes());
    println!("Got compile calldata {}", hex::encode(&compile_calldata));

    let to = hex::decode(constants::ARB_WASM_ADDRESS).unwrap();
    let to = H160::from_slice(&to);
    let tx = Eip1559TransactionRequest::new()
        .from(addr)
        .to(to)
        .max_priority_fee_per_gas(base_fee)
        .data(compile_calldata);
    let tx = TypedTransaction::Eip1559(tx);

    println!("Sending compile tx");
    let pending_tx = client.send_transaction(tx, None).await?;

    let receipt = pending_tx
        .await?
        .ok_or_else(|| eyre::format_err!("Tx dropped from mempool"))?;

    println!("Tx receipt: {}", serde_json::to_string(&receipt)?);
    Ok(())
}
