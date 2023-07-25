// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use std::env::current_dir;
use std::io::Read;
use std::path::{Component, PathBuf};
use std::process::{Command, Stdio};

use brotli2::read::BrotliEncoder;
use bytes::buf::Reader;
use bytes::Buf;

use crate::constants;
use arbutil::Color;

/// Loads the project name from the current working directory,
/// which is assumed to be the project root.
pub fn get_project_name(cwd: &PathBuf) -> Option<String> {
    while let Some(component) = cwd.components().into_iter().next() {
        match component {
            Component::Normal(name) => {
                return Some(name.to_str().unwrap().to_string());
            }
            _ => {}
        }
    }
    None
}

/// Build a Rust project to WASM and return the path to the compiled WASM file.
/// TODO: Configure debug or release via flags.
pub fn build_project_to_wasm() -> eyre::Result<PathBuf, String> {
    let cwd: PathBuf = current_dir().map_err(|e| format!("Could not get current dir {}", e))?;
    let project_name = get_project_name(&cwd).ok_or("Could not get project name from directory")?;

    Command::new("cargo")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .arg("build")
        .arg("--release")
        .arg("--target=wasm32-unknown-unknown")
        .output()
        .map_err(|e| format!("Failed to execute cargo build {}", e))?;

    let wasm_path = cwd
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{}.wasm", project_name));
    Ok(wasm_path)
}

/// Reads a WASM file at a specified path and returns its brotli compressed bytes.
pub fn get_compressed_wasm_bytes(wasm_path: &PathBuf) -> eyre::Result<Vec<u8>, String> {
    println!("Reading WASM file at {}", wasm_path.display().yellow());

    let wasm_file_bytes = std::fs::read(&wasm_path)
        .map_err(|e| format!("Could not read WASM file at target path {}", e))?;
    let wbytes: Reader<&[u8]> = wasm_file_bytes.reader();

    let mut compressor = BrotliEncoder::new(wbytes, constants::BROTLI_COMPRESSION_LEVEL);
    let mut compressed_bytes = vec![];
    compressor
        .read_to_end(&mut compressed_bytes)
        .map_err(|e| format!("Could not Brotli compress WASM bytes {}", e))?;

    println!(
        "Compressed WASM size: {} bytes",
        compressed_bytes.len().to_string().yellow()
    );
    let mut code = hex::decode(constants::EOF_PREFIX).unwrap();
    code.extend(compressed_bytes);
    Ok(code)
}
