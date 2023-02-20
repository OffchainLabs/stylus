use crate::config::*;
use crate::consts::*;
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

// TODO: Checks if we are inside a stylus project (is there a stylus.toml)
// and from there, runs the cargo compilation toolchain into WASM.
pub fn compile_project(project_name: String) {
    let cwd: PathBuf = current_dir().unwrap();
    let stylus_config_path = cwd.join(&project_name).join(PROJECT_CONFIG_FILE);

    if !stylus_config_path.exists() {
        panic!("No Stylus config file found - not a Stylus project");
    }

    let manifest_path = cwd
        .join(&project_name)
        .join("Cargo.toml")
        .into_os_string()
        .into_string()
        .unwrap();

    // TODO: Configure debug or release via flags.
    // TODO: Capture errors from this command.
    Command::new("cargo")
        .arg("build")
        .arg(format!("--manifest-path={}", manifest_path)) // TODO: Brittle.
        .arg("--target=wasm32-unknown-unknown")
        .output()
        .expect("Failed to execute cargo build");

    let wasm_path = cwd
        .join(&project_name)
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("debug")
        .join(format!("{}.wasm", &project_name));

    println!("Reading compiled WASM at {}", wasm_path.display().yellow());
    let wasm_file_bytes =
        std::fs::read(&wasm_path).expect("Could not read WASM file at target path");

    let wbytes: Reader<&[u8]> = wasm_file_bytes.reader();
    let mut compressor = BrotliEncoder::new(wbytes, 9);
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

    let stylus_path = cwd
        .join(&project_name)
        .join("target")
        .join(STYLUS_TARGET_SUBFOLDER);

    if !stylus_path.exists() {
        std::fs::create_dir(&stylus_path).expect("Could not create stylus subfolder");
    }

    let compressed_output_path =
        stylus_path.join(format!("{}{}", &project_name, COMPRESSED_WASM_EXT));

    println!(
        "Writing compressed WASM to file {}",
        compressed_output_path.display().yellow()
    );

    // Prepend the EOF_PREFIX on the brotli-compressed WASM.
    // TODO: Abstract this out.
    let eof = hex::decode(EOF_PREFIX).unwrap();
    let compressed_bytes: Vec<u8> = compressed_bytes.splice(0..0, eof).collect();

    std::fs::write(compressed_output_path, &compressed_bytes)
        .expect("Could not write compressed WASM to output folder");

    println!("Instrumenting with Stylus...");

    // TODO: Move this logic into config.rs
    let config_file_bytes =
        std::fs::read(&stylus_config_path).expect("Could not read Stylus config file");
    let config_file_str =
        std::str::from_utf8(&config_file_bytes).expect("Could not read file as string");
    let project_config: ProjectConfig =
        toml::from_str(config_file_str).expect("Invalid stylus config");
    let config = StylusConfig::from(project_config);

    let instrumented = stylus::native::module(&wasm_file_bytes, config.clone()).unwrap();

    let instrumented_output_path =
        stylus_path.join(format!("{}{}", &project_name, INSTRUMENTED_WASM_EXT));

    println!(
        "Writing instrumented module to file {}",
        instrumented_output_path.display().yellow()
    );
    std::fs::write(instrumented_output_path, &instrumented)
        .expect("Could not write instrumented WASM to output");
    println!("Done!");
}
