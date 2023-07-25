// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use std::convert::TryFrom;

use crate::constants;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Eip1559TransactionRequest, U256};
use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
};

/// Submits a signed tx to an endpoint, given a wallet, a data payload, and a closure
/// to get a transaction request to sign and send. If estimate_only is true, only a call to
/// estimate gas will occur and the actual tx will not be submitted.
pub async fn submit_signed_tx<F>(
    endpoint: &str,
    wallet: LocalWallet,
    estimate_only: bool,
    prep_tx: F,
) -> eyre::Result<(), String>
where
    F: FnOnce(U256) -> Eip1559TransactionRequest,
{
    let provider = Provider::<Http>::try_from(endpoint)
        .map_err(|e| format!("could not initialize provider from http {}", e))?;
    let chain_id = provider
        .get_chainid()
        .await
        .map_err(|e| format!("could not get chain id {}", e))?
        .as_u64();
    let addr = wallet.address();
    let client = SignerMiddleware::new(provider, wallet.with_chain_id(chain_id));

    let nonce = client
        .get_transaction_count(addr, None)
        .await
        .map_err(|e| format!("Could not get nonce {} {}", addr, e))?;
    let block_num = client
        .get_block_number()
        .await
        .map_err(|e| format!("Could not get block number {}", e))?;
    let block = client
        .get_block(block_num)
        .await
        .map_err(|e| format!("Could not get block {}", e))?
        .ok_or("No block found")?;
    let base_fee = block.base_fee_per_gas.expect("No base fee found for block");

    let to = hex::decode(constants::MULTICALL_ADDR).unwrap();
    let tx = prep_tx(base_fee);
    let typed = TypedTransaction::Eip1559(tx.clone());
    let estimated = client
        .estimate_gas(&typed, None)
        .await
        .map_err(|e| format!("{}", e))?;

    println!("Estimated gas: {estimated}");

    if estimate_only {
        return Ok(());
    }

    println!("Submitting tx...");
    let pending_tx = client
        .send_transaction(tx, None)
        .await
        .map_err(|e| format!("Could not send tx {}", e))?;

    let receipt = pending_tx
        .await
        .map_err(|e| format!("Could not get receipt {}", e))?
        .ok_or("No receipt found")?;

    match receipt.status {
        None => Err(format!(
            "Tx with hash {} reverted",
            receipt.transaction_hash
        )),
        Some(_) => Ok(()),
    }
}
