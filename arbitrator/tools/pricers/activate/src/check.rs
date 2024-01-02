// Copyright 2022-2024, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use activate::{Trial, util};
use eyre::Result;
use std::path::PathBuf;

pub fn check(path: PathBuf) -> Result<()> {
    let wat = util::file_bytes(&path)?;
    let wasm = wasmer::wat2wasm(&wat)?;
    let trial = Trial::new(&wasm)?;
    trial.print();
    trial.check_model(1.)
}
