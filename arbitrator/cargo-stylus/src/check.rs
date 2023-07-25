// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use bytesize::ByteSize;

use arbutil::Color;
use prover::programs::prelude::*;

use crate::constants;
use crate::project;

/// Defines the stylus checks that occur during the compilation of a WASM program
/// into a module. Checks can be disabled during the compilation process for debugging purposes.
#[derive(PartialEq)]
pub enum StylusCheck {
    CompressedSize,
    // TODO: Adding more checks here would require being able to toggle
    // compiler middlewares in the compile config store() method.
}

impl TryFrom<&str> for StylusCheck {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "compressed-size" => Ok(StylusCheck::CompressedSize),
            _ => Err(format!("invalid Stylus middleware check: {}", value,)),
        }
    }
}

/// Runs a series of checks on the WASM program to ensure it is valid for compilation
/// and code size before being deployed and compiled onchain. An optional list of checks
/// to disable can be specified.
pub fn run_checks(disabled: Option<Vec<StylusCheck>>) -> eyre::Result<(), String> {
    let wasm_file_path = project::build_project_to_wasm()?;
    let wasm_file_bytes = project::get_compressed_wasm_bytes(&wasm_file_path)?;
    println!(
        "Compressed WASM size: {}",
        ByteSize::b(wasm_file_bytes.len() as u64)
            .to_string()
            .yellow(),
    );

    let compressed_size = ByteSize::b(wasm_file_bytes.len() as u64);
    let check_compressed_size = disabled
        .as_ref()
        .map(|d: &Vec<StylusCheck>| !d.contains(&StylusCheck::CompressedSize))
        .unwrap_or(true);

    if check_compressed_size && compressed_size > constants::MAX_PROGRAM_SIZE {
        return Err(format!(
            "Brotli-compressed WASM size {} is bigger than program size limit: {}",
            compressed_size.to_string().red(),
            constants::MAX_PROGRAM_SIZE,
        ));
    }
    compile_native_wasm_module(CompileConfig::default(), &wasm_file_bytes)?;
    Ok(())
}

/// Compiles compressed wasm file bytes into a native module using a specified compile config.
pub fn compile_native_wasm_module(
    cfg: CompileConfig,
    wasm_file_bytes: &[u8],
) -> eyre::Result<Vec<u8>, String> {
    let module = stylus::native::module(&wasm_file_bytes, cfg)
        .map_err(|e| format!("could not compile wasm {}", e))?;
    let success = "Stylus compilation successful!".to_string().mint();
    println!("{}", success);

    println!(
        "Compiled WASM module total size: {}",
        ByteSize::b(module.len() as u64).to_string()
    );
    Ok(module)
}
