// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use bytesize::ByteSize;

use arbutil::Color;
use prover::programs::prelude::*;

use crate::constants;

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
pub fn run_checks(wasm_file_bytes: &[u8], disabled: Vec<StylusCheck>) -> eyre::Result<(), String> {
    println!(
        "Compressed WASM size: {}",
        ByteSize::b(wasm_file_bytes.len() as u64)
            .to_string()
            .yellow(),
    );

    let compressed_size = ByteSize::b(wasm_file_bytes.len() as u64);
    let check_compressed_size = disabled.contains(&StylusCheck::CompressedSize);

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
    let module = stylus::native::module(wasm_file_bytes, cfg)
        .map_err(|e| format!("Could not compile wasm {}", e))?;
    let success = "Stylus compilation successful!".to_string().mint();
    println!("{}", success);

    println!(
        "Compiled WASM module total size: {}",
        ByteSize::b(module.len() as u64),
    );
    Ok(module)
}

#[cfg(test)]
mod test {
    use super::*;
    use wasmer::wat2wasm;
    #[test]
    fn test_run_checks() {
        let wat = r#"
        (module
            (func $foo (export "foo") (result v128)
                v128.const i32x4 1 2 3 4))
        "#;
        let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();
        let disabled = vec![];
        match run_checks(&wasm_bytes, disabled) {
            Ok(_) => panic!("Expected error"),
            Err(e) => assert!(e.contains("128-bit types are not supported")),
        }
    }
}
