// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use std::env::current_dir;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use brotli2::read::BrotliEncoder;
use bytesize::ByteSize;

use crate::{
    color::Color,
    constants::{BROTLI_COMPRESSION_LEVEL, EOF_PREFIX, RUST_TARGET},
};

/// Build a Rust project to WASM and return the path to the compiled WASM file.
pub fn build_project_to_wasm() -> eyre::Result<PathBuf, String> {
    let cwd: PathBuf = current_dir().map_err(|e| format!("could not get current dir: {e}"))?;

    Command::new("cargo")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .arg("build")
        .arg("--release")
        .arg(format!("--target={}", RUST_TARGET))
        .output()
        .map_err(|e| format!("failed to execute cargo build: {e}"))?;

    let release_path = cwd.join("target").join(RUST_TARGET).join("release");

    // Gets the files in the release folder.
    let release_files: Vec<PathBuf> = std::fs::read_dir(release_path)
        .map_err(|e| format!("could not read release dir: {e}"))?
        .filter(|r| r.is_ok())
        .map(|r| r.unwrap().path())
        .filter(|r| r.is_file())
        .collect();

    let wasm_file_path = release_files
        .into_iter()
        .find(|p| {
            if let Some(ext) = p.file_name() {
                return ext.to_str().unwrap_or("").contains(".wasm");
            }
            false
        })
        .ok_or("could not find WASM file in release dir")?;
    Ok(wasm_file_path)
}

/// Reads a WASM file at a specified path and returns its brotli compressed bytes.
pub fn get_compressed_wasm_bytes(wasm_path: &PathBuf) -> eyre::Result<(Vec<u8>, Vec<u8>), String> {
    println!("Reading WASM file at {}", wasm_path.display().grey());

    let wasm_file_bytes = std::fs::read(wasm_path).map_err(|e| {
        format!(
            "could not read WASM file at target path {}: {e}",
            wasm_path.as_os_str().to_string_lossy(),
        )
    })?;

    let wasm_bytes = wasmer::wat2wasm(&wasm_file_bytes)
        .map_err(|e| format!("could not parse wasm file bytes: {e}"))?;
    let wasm_bytes = &*wasm_bytes;

    let mut compressor = BrotliEncoder::new(wasm_bytes, BROTLI_COMPRESSION_LEVEL);
    let mut compressed_bytes = vec![];
    compressor
        .read_to_end(&mut compressed_bytes)
        .map_err(|e| format!("could not Brotli compress WASM bytes: {e}"))?;

    println!(
        "Compressed WASM size: {}",
        ByteSize::b(compressed_bytes.len() as u64)
            .to_string()
            .mint(),
    );
    let mut deploy_ready_code = hex::decode(EOF_PREFIX).unwrap();
    deploy_ready_code.extend(compressed_bytes);
    Ok((wasm_bytes.to_vec(), deploy_ready_code))
}
