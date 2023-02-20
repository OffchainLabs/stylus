use clap::{Parser, Subcommand};

mod compile;
mod config;
mod consts;
mod new;
mod run;

#[derive(Parser, Debug)]
#[command(name = "stylus")]
#[command(author = "Offchain Labs, Inc.")]
#[command(version = "0.0.1")]
#[command(about = "Command-line tool for stylus projects on Arbitrum", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize a new stylus project with a specified name
    #[command(alias = "n")]
    New { project_name: String },
    /// Compile a Rust project using Stylus, outputting both Brotli-compressed WASM ready
    /// to deploy on-chain, and also the instrumented machine's WASM
    #[command(alias = "c")]
    Compile { project_name: String },
    /// Run a compiled Stylus WASM natively
    #[command(alias = "r")]
    Run {
        project_name: String,
        input_data_hex: Option<String>,
    },
    /// Deploy a stylus project to an Arbitrum chain
    #[command(alias = "d")]
    Deploy { project_name: String },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::New { project_name } => new::new_project(project_name.to_string()),
        Commands::Compile { project_name } => compile::compile_project(project_name.to_string()),
        Commands::Run {
            project_name,
            input_data_hex,
        } => run::run_project(project_name.to_string(), input_data_hex.clone()),
        Commands::Deploy { .. } => {
            todo!();
        }
    }
}
