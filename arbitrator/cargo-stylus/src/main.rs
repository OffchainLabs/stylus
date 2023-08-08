use std::path::PathBuf;
use std::str::FromStr;

use check::StylusCheck;
// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use clap::{Args, Parser, Subcommand, ValueEnum};
use ethers::types::H160;

mod check;
mod constants;
mod deploy;
mod project;
mod tx;

#[derive(Parser, Debug)]
#[command(name = "stylus")]
#[command(author = "Offchain Labs, Inc.")]
#[command(version = "0.0.1")]
#[command(about = "Cargo command for developing Arbitrum Stylus projects", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Instrument a Rust project using Stylus,
    /// . This command runs compiled WASM code through
    /// Stylus instrumentation checks and reports any failures. Allows for disabling specific.
    /// checks via the `--disabled-checks` flag.
    #[command(alias = "c")]
    Check {
        /// Disables specific compilation checks. At the moment, `compressed-size` is the only
        /// option available to disable. Disabling it skips checking the compressed program
        /// is within the 24Kb contract limit.
        #[arg(long)]
        disabled_checks: Option<Vec<String>>,
        #[arg(long)]
        wasm_file_path: Option<String>,
    },
    /// Instruments a Rust project using Stylus and by outputting its brotli-compressed WASM code.
    /// Then, it submits two transactions: the first deploys the WASM
    /// program to an address and the second triggers a compilation onchain
    /// Developers can choose to split up the deploy and compile steps via this command as desired.
    #[command(alias = "d")]
    Deploy(DeployConfig),
}

#[derive(Debug, Args)]
pub struct DeployConfig {
    /// Does not submit a transaction, but instead estimates the gas required
    /// to complete the operation.
    #[arg(long)]
    estimate_gas_only: bool,
    /// By default, submits a single, atomic deploy and compile transaction to Arbitrum.
    /// Otherwise, a user could choose to split up the deploy and compile steps into individual transactions.
    #[arg(long, value_enum)]
    mode: Option<DeployMode>,
    /// The endpoint of the L2 node to connect to.
    #[arg(short, long, default_value = "http://localhost:8545")]
    endpoint: String,
    /// Wallet source to use with the cargo stylus plugin.
    #[command(flatten)]
    wallet: WalletSource,
    /// If only compiling an onchain program, the address of the program to send a compilation tx for.
    #[arg(long)]
    compile_program_address: Option<H160>,
    /// If desired, it loads a WASM file from a specified path. If not provided, it will try to find
    /// a WASM file under the current working directory's Rust target release directory and use its
    /// contents for the deploy command.
    #[arg(long)]
    wasm_file_path: Option<String>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum DeployMode {
    DeployOnly,
    CompileOnly,
}

#[derive(Clone, Debug, Args)]
#[group(required = true, multiple = false)]
pub struct WalletSource {
    #[arg(long, group = "keystore")]
    keystore_path: Option<String>,
    #[arg(long, group = "keystore")]
    keystore_password_path: Option<String>,
    #[arg(long)]
    private_key_path: Option<String>,
}

#[tokio::main]
async fn main() -> eyre::Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check {
            disabled_checks,
            wasm_file_path,
        } => {
            let disabled = disabled_checks.map_or(Vec::default(), |checks| {
                checks
                    .into_iter()
                    .map(|s| s.as_str().try_into())
                    .collect::<Result<Vec<StylusCheck>, String>>()
                    .expect("Could not parse disabled Stylus checks")
            });
            let wasm_file_path: PathBuf = match wasm_file_path {
                Some(path) => PathBuf::from_str(&path).unwrap(),
                None => project::build_project_to_wasm()?,
            };
            let wasm_file_bytes = project::get_compressed_wasm_bytes(&wasm_file_path)?;
            check::run_checks(&wasm_file_bytes, disabled)
        }
        Commands::Deploy(deploy_config) => match deploy::deploy(deploy_config).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!(
                "Could not perform deployment/compilation transaction {}",
                e
            )),
        },
    }
}
