use crate::config::*;
use crate::consts::*;
use std::env::current_dir;
use std::path::PathBuf;

use arbutil::Color;
use prover::programs::prelude::*;
use stylus::{native::NativeInstance, run::RunProgram};

use hex;

pub fn run_project(project_name: String, input_data_hex: Option<String>) {
    let cwd: PathBuf = current_dir().expect("Could not read current directory");
    let stylus_config_path = cwd.join(&project_name).join(PROJECT_CONFIG_FILE);

    if !stylus_config_path.exists() {
        panic!("No Stylus config file found - not a Stylus project");
    }

    let args = match input_data_hex {
        Some(data) => {
            let mut args = vec![0x01];
            let input_data = hex::decode(data).expect("Could not decode input data hex");
            args.extend(input_data);
            args
        }
        None => vec![],
    };

    // TODO: Customize release or debug flags
    let wasm_path = cwd
        .join(project_name.clone())
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("debug")
        .join(format!("{}.wasm", project_name));

    let wasm_path_str = wasm_path.clone().into_os_string().into_string().unwrap();

    // TODO: Move to config.rs
    let config_file_bytes =
        std::fs::read(&stylus_config_path).expect("Could not read Stylus config file");
    let config_file_str =
        std::str::from_utf8(&config_file_bytes).expect("Could not read file as string");
    let project_config: ProjectConfig =
        toml::from_str(config_file_str).expect("Invalid stylus config");
    let config = StylusConfig::from(project_config);

    // TODO: Should use a pathbuf instead.
    let mut native = NativeInstance::from_path(&wasm_path_str, &config).unwrap();
    match native.run_main(&args, &config).unwrap() {
        UserOutcome::Success(output) => {
            println!("Got native output {}", hex::encode(output).mint());
        }
        err => panic!("User program failure: {}", err.red()),
    }
}
