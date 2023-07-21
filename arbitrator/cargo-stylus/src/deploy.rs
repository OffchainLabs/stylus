use std::convert::TryFrom;
use std::env::current_dir;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use brotli2::read::BrotliEncoder;
use bytes::buf::Reader;
use bytes::{Buf, Bytes};

use ethers::types::{Address, H160};
use ethers::{
    core::{types::TransactionRequest, utils::Anvil},
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
};

use arbutil::Color;

use crate::constants;

pub async fn deploy_and_compile_onchain() -> eyre::Result<()> {
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

    let wbytes: Reader<&[u8]> = wasm_file_bytes.reader();

    let mut compressor = BrotliEncoder::new(wbytes, constants::BROTLI_COMPRESSION_LEVEL);
    let mut compressed_bytes = vec![];
    compressor.read_to_end(&mut compressed_bytes).unwrap();

    // TODO: Add the compression and compilation checks in here. Reuse functions from check.

    // Next, we prepend with the EOF bytes and prepare a compilation tx onchain. Uses ethers
    // to prepare the tx and send it over onchain to an endpoint. Will prepare a multicall data
    // tx to send to a multicall.rs rust program.
    Ok(())
}

async fn submit_signed_tx(endpoint: &str) -> eyre::Result<()> {
    let anvil = Anvil::new().spawn();

    let wallet: LocalWallet = anvil.keys()[0].clone().into();
    let addr = wallet.address();
    let provider = Provider::<Http>::try_from(endpoint)?;
    let client = SignerMiddleware::new(provider, wallet.with_chain_id(anvil.chain_id()));

    let tx = prepare_tx(addr, Bytes::default());
    let pending_tx = client.send_transaction(tx, None).await?;

    let receipt = pending_tx
        .await?
        .ok_or_else(|| eyre::format_err!("tx dropped from mempool"))?;

    let tx = client.get_transaction(receipt.transaction_hash).await?;

    println!("Sent tx: {}\n", serde_json::to_string(&tx)?);
    println!("Tx receipt: {}", serde_json::to_string(&receipt)?);
    Ok(())
}

fn prepare_tx(address: H160, data: Bytes) -> TransactionRequest {
    TransactionRequest::new()
        .to(address)
        .data(data)
}

fn prepare_compilation_tx() -> eyre::Result<()> {
    Ok(())
}
