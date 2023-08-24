use std::io::{BufRead, BufReader};
use std::str::FromStr;

use ethers::signers::LocalWallet;

use crate::KeystoreOpts;

/// Loads a wallet for signing transactions either from a private key file path.
/// or a keystore along with a keystore password file.
pub fn load(
    private_key_path: Option<String>,
    keystore_opts: KeystoreOpts,
) -> eyre::Result<LocalWallet, String> {
    if private_key_path.is_some()
        && (keystore_opts.keystore_password_path.is_some() && keystore_opts.keystore_path.is_some())
    {
        return Err("must provide either --private-key-path or (--keystore-path and --keystore-password-path)".to_string());
    }

    if let Some(priv_key_path) = &private_key_path {
        let privkey = read_secret_from_file(&priv_key_path)?;
        return LocalWallet::from_str(&privkey)
            .map_err(|e| format!("could not parse private key: {e}"));
    }
    let keystore_password_path = keystore_opts
        .keystore_password_path
        .as_ref()
        .ok_or("no keystore password path provided")?;
    let keystore_pass = read_secret_from_file(&keystore_password_path)?;
    let keystore_path = keystore_opts
        .keystore_path
        .as_ref()
        .ok_or("no keystore path provided")?;
    LocalWallet::decrypt_keystore(keystore_path, keystore_pass)
        .map_err(|e| format!("could not decrypt keystore: {e}"))
}

fn read_secret_from_file(fpath: &str) -> Result<String, String> {
    let f = std::fs::File::open(fpath)
        .map_err(|e| format!("could not open file at path: {fpath}: {e}"))?;
    let mut buf_reader = BufReader::new(f);
    let mut secret = String::new();
    buf_reader
        .read_line(&mut secret)
        .map_err(|e| format!("could not read secret from file at path {fpath}: {e}"))?;
    Ok(secret.trim().to_string())
}
