use std::env::current_dir;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

use brotli2::read::BrotliEncoder;
use bytes::buf::Reader;
use bytes::Buf;
use bytesize::ByteSize;
use hex;

use arbutil::Color;
use prover::programs::prelude::*;

use crate::constants;

#[derive(PartialEq)]
pub enum StylusCheck {
    CompressedSize,
    // TODO: Adding more checks here would require being able to toggle
    // compiler middlewares in the compile config store() method.
}

impl From<&str> for StylusCheck {
    fn from(value: &str) -> Self {
        match value {
            "compressed-size" => StylusCheck::CompressedSize,
            _ => panic!(
                "Invalid Stylus middleware check: {}, allowed middlewares are: foo",
                value
            ),
        }
    }
}

pub fn run_checks(disabled: Option<Vec<StylusCheck>>) -> eyre::Result<()> {
    let cwd: PathBuf = current_dir().unwrap();

    // Compile the Rust program at the current working directory into WASM using
    // Cargo and then instrument the WASM code with Stylus. If any of the checks
    // are disabled, we avoid runnng it.
    let _check_compressed_size = disabled
        .as_ref()
        .map(|d: &Vec<StylusCheck>| !d.contains(&StylusCheck::CompressedSize))
        .unwrap_or(true);

    // TODO: Configure debug or release via flags.
    // TODO: Capture errors from this command.
    Command::new("cargo")
        .arg("build")
        .arg("--target=wasm32-unknown-unknown")
        .output()
        .expect("Failed to execute cargo build");

    let wasm_path = cwd
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("debug")
        .join(format!("{}.wasm", "echo"));

    println!("Reading compiled WASM at {}", wasm_path.display().yellow());

    let wasm_file_bytes =
        std::fs::read(&wasm_path).expect("Could not read WASM file at target path");

    let wbytes: Reader<&[u8]> = wasm_file_bytes.reader();

    // TODO: Configure compression level, move to constants.
    let mut compressor = BrotliEncoder::new(wbytes, constants::BROTLI_COMPRESSION_LEVEL);
    let mut compressed_bytes = vec![];
    compressor.read_to_end(&mut compressed_bytes).unwrap();

    println!(
        "Uncompressed size: {}",
        ByteSize::b(wasm_file_bytes.len() as u64)
            .to_string()
            .yellow(),
    );

    println!(
        "Brotli compressed size: {}",
        ByteSize::b(compressed_bytes.len() as u64)
            .to_string()
            .mint(),
    );

    let config = CompileConfig::default();
    let instrumented = stylus::native::module(
        &wasm_file_bytes, config
    ).unwrap();

    println!(
        "Instrumented wasm raw bytes: {}",
        ByteSize::b(instrumented.len() as u64)
            .to_string()
            .mint(),
    );

    Ok(())
}