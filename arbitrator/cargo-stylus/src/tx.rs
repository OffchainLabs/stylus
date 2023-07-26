// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::Eip1559TransactionRequest;
use ethers::{middleware::SignerMiddleware, providers::Middleware, signers::Signer};

/// Submits a signed tx to an endpoint, given a wallet, a data payload, and a closure
/// to get a transaction request to sign and send. If estimate_only is true, only a call to
/// estimate gas will occur and the actual tx will not be submitted.
pub async fn submit_signed_tx<M, S>(
    client: SignerMiddleware<M, S>,
    estimate_only: bool,
    tx_request: &mut Eip1559TransactionRequest,
) -> eyre::Result<(), String>
where
    M: Middleware,
    S: Signer,
{
    let block_num = client
        .get_block_number()
        .await
        .map_err(|e| format!("could not get block number {}", e))?;
    let block = client
        .get_block(block_num)
        .await
        .map_err(|e| format!("could not get block {}", e))?
        .ok_or("no block found")?;
    let base_fee = block
        .base_fee_per_gas
        .ok_or("no base fee found for block")?;

    tx_request.max_fee_per_gas = Some(base_fee);

    let typed = TypedTransaction::Eip1559(tx_request.clone());
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
        .send_transaction(typed, None)
        .await
        .map_err(|e| format!("could not send tx {}", e))?;

    let receipt = pending_tx
        .await
        .map_err(|e| format!("could not get receipt {}", e))?
        .ok_or("no receipt found")?;

    match receipt.status {
        None => Err(format!(
            "Tx with hash {} reverted",
            receipt.transaction_hash
        )),
        Some(_) => Ok(()),
    }
}
