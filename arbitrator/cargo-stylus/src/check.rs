use std::env::current_dir;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use brotli2::read::BrotliEncoder;
use bytes::buf::Reader;
use bytes::Buf;
use bytesize::ByteSize;
use eyre::bail;

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

// TODO: separate out the business logic of cargo compilation, reading the file, etc.
// as it will be reused by the deploy command.
pub fn run_checks(disabled: Option<Vec<StylusCheck>>) -> eyre::Result<()> {
    // Compile the Rust program at the current working directory into WASM using
    // Cargo and then instrument the WASM code with Stylus. If any of the checks
    // are disabled, we avoid runnng it.
    let check_compressed_size = disabled
        .as_ref()
        .map(|d: &Vec<StylusCheck>| !d.contains(&StylusCheck::CompressedSize))
        .unwrap_or(true);

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

    // TODO: Configure compression level.
    let mut compressor = BrotliEncoder::new(wbytes, constants::BROTLI_COMPRESSION_LEVEL);
    let mut compressed_bytes = vec![];
    compressor.read_to_end(&mut compressed_bytes).unwrap();

    println!(
        "Uncompressed WASM size: {}",
        ByteSize::b(wasm_file_bytes.len() as u64)
            .to_string()
            .yellow(),
    );

    let compressed_size = ByteSize::b(compressed_bytes.len() as u64);

    if check_compressed_size {
        // TODO: Configure.
        if compressed_size > ByteSize::kb(24) {
            bail!(
                "Brotli-compressed WASM size {} is bigger than program size limit: {}",
                compressed_size.to_string().red(),
                ByteSize::kb(24).to_string(),
            );
        } else {
            println!(
                "Brotli-compressed WASM size {} within program size limit: {}",
                compressed_size.to_string().mint(),
                ByteSize::kb(24).to_string(),
            );
        }
    }

    let config = CompileConfig::default();
    let module = match stylus::native::module(&wasm_file_bytes, config) {
        Ok(module) => module,
        Err(error) => {
            bail!("Failed to compile WASM: {:?}", error);
        }
    };

    let success = "Stylus compilation successful!".to_string().mint();
    println!("{}", success);
    println!(
        "Compiled WASM module total size: {}",
        ByteSize::b(module.len() as u64).to_string()
    );
    Ok(())
}

pub fn check_compressed_size() -> eyre::Result<()> {
    Ok(())
}

pub fn check_compilation() -> eyre::Result<()> {
    Ok(())
}
