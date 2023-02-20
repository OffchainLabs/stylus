use crate::config::*;
use crate::consts::*;
use std::env::current_dir;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;

use arbutil::Color;

use toml;

// TODO: Writes a stylus toml with some default config options.
pub fn new_project(project_name: String) {
    let cwd: PathBuf = current_dir().expect("Could not read current directory");
    let project_path = cwd.join(&project_name);

    if project_path.exists() {
        panic!(
            "Project with name {} already exists",
            &project_name.yellow()
        );
    }

    Command::new("cargo")
        .arg("new")
        .arg(&project_name)
        .arg("--bin")
        .output()
        .expect("failed to execute process");

    // Write the project configuration
    let default_cfg = ProjectConfig::default();
    let cfg = toml::to_string(&default_cfg).expect("Could not serialize configuration");

    println!("Writing Stylus configuration with defaults");
    println!("{}", cfg.mint());

    let config_path = project_path.join(PROJECT_CONFIG_FILE);
    std::fs::write(&config_path, cfg.as_bytes()).expect("Could not write config file");

    // Overwrite the main.rs file
    let main_path = project_path.join("src").join("main.rs");
    std::fs::write(&main_path, main_template().as_bytes()).expect("Could not write main.rs");

    // Add extra lines to Cargo.toml
    let cargo_path = project_path.join("Cargo.toml");
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(cargo_path)
        .unwrap();

    file.write_all(cargo_append_template().as_bytes())
        .expect("Could not append to Cargo.toml");

    println!(
        "Created new Stylus project at {}",
        project_path.display().mint()
    );
}

fn cargo_append_template() -> String {
    r#"
[workspace]

[profile.release]
codegen-units = 1
strip = true
lto = true
panic = "abort"
    "#
    .to_string()
}

fn main_template() -> String {
    r#"// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

#![no_main]

arbitrum_main!(user_main);

fn user_main(input: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    Ok(input)
}

#[link(wasm_import_module = "forward")]
extern "C" {
    pub fn read_args(dest: *mut u8);
    pub fn return_data(data: *const u8, len: usize);
}

pub fn args(len: usize) -> Vec<u8> {
    let mut input = Vec::with_capacity(len);
    unsafe {
        read_args(input.as_mut_ptr());
        input.set_len(len);
    }
    input
}

pub fn output(data: Vec<u8>) {
    unsafe {
        return_data(data.as_ptr(), data.len());
    }
}

#[macro_export]
macro_rules! arbitrum_main {
    ($name:expr) => {
        #[no_mangle]
        pub extern "C" fn arbitrum_main(len: usize) -> usize {
            let input = args(len);
            let (data, status) = match $name(input) {
                Ok(data) => (data, 0),
                Err(data) => (data, 1),
            };
            output(data);
            status
        }
    };
}"#
    .to_string()
}
