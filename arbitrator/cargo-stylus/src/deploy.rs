use std::str::FromStr;

use ethers::{signers::LocalWallet, types::U256};

use crate::WalletSource;

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
    println!("Got prelude={prelude}");
    let first_few: [u8; 12] = code[..12].try_into().unwrap();
    let first_few = hex::encode(first_few);
    println!("First 12 bytes={first_few}");
    deploy.extend(code);
    deploy
}

// let multicall_data = prepare_deploy_compile_multicall(&code, &contract_addr);

// let to = hex::decode(constants::MULTICALL_ADDR).unwrap();
// let tx = Eip1559TransactionRequest::new()
//     .from(addr)
//     .to(H160::from_slice(&to))
//     .max_priority_fee_per_gas(base_fee)
//     .data(multicall_data);
//let init_code = contract_init_code(&code);
// let tx = Eip1559TransactionRequest::new()
//     .from(addr)
//     .max_priority_fee_per_gas(base_fee)
//     .data(init_code);
// let tx = TypedTransaction::Eip1559(tx);

// let mut compile_calldata = vec![];
// let compile_method_hash = hex::decode("2e50f32b").unwrap();
// compile_calldata.extend(compile_method_hash);
// compile_calldata.extend(hex::decode("000000000000000000000000").unwrap());
// compile_calldata.extend(contract_addr.as_bytes());
// println!("Got compile calldata {}", hex::encode(&compile_calldata));

// let to = hex::decode(constants::ARB_WASM_ADDRESS).unwrap();
// let to = H160::from_slice(&to);
// let tx = Eip1559TransactionRequest::new()
//     .from(addr)
//     .to(to)
//     .max_priority_fee_per_gas(base_fee)
//     .data(compile_calldata);
// let tx = TypedTransaction::Eip1559(tx);
