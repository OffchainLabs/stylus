// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use clap::{Args, Parser, ValueEnum};
use color::Color;
use ethers::types::H160;

mod check;
mod color;
mod constants;
mod deploy;
mod project;
mod tx;
mod wallet;

#[derive(Parser, Debug)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    Stylus(StylusArgs),
}

#[derive(Parser, Debug)]
#[command(name = "stylus")]
#[command(bin_name = "cargo stylus")]
#[command(author = "Offchain Labs, Inc.")]
#[command(version = "0.0.1")]
#[command(about = "Cargo command for developing Arbitrum Stylus projects", long_about = None)]
#[command(propagate_version = true)]
struct StylusArgs {
    #[command(subcommand)]
    command: StylusSubcommands,
}

#[derive(Parser, Debug, Clone)]
enum StylusSubcommands {
    /// Instrument a Rust project using Stylus.
    /// This command runs compiled WASM code through Stylus instrumentation checks and reports any failures.
    #[command(alias = "c")]
    Check(CheckConfig),
    /// Instruments a Rust project using Stylus and by outputting its brotli-compressed WASM code.
    /// Then, it submits two transactions: the first deploys the WASM
    /// program to an address and the second triggers an activation onchain
    /// Developers can choose to split up the deploy and activate steps via this command as desired.
    #[command(alias = "d")]
    Deploy(DeployConfig),
}

#[derive(Debug, Args, Clone)]
pub struct CheckConfig {
    /// The endpoint of the L2 node to connect to.
    #[arg(short, long, default_value = "http://localhost:8545")]
    endpoint: String,
    /// If desired, it loads a WASM file from a specified path. If not provided, it will try to find
    /// a WASM file under the current working directory's Rust target release directory and use its
    /// contents for the deploy command.
    #[arg(long)]
    wasm_file_path: Option<String>,
    /// Specify the program address we want to check activation for. If unspecified, it will
    /// compute the next program address from the user's wallet address and nonce.
    /// To avoid needing a wallet to run this command, pass in 0x0000000000000000000000000000000000000000
    /// or any other desired program address to check against.
    #[arg(long)]
    activate_program_address: Option<H160>,
    /// Privkey source to use with the cargo stylus plugin.
    #[arg(long)]
    private_key_path: Option<String>,
    /// Wallet source to use with the cargo stylus plugin.
    #[command(flatten)]
    keystore_opts: KeystoreOpts,
}

#[derive(Debug, Args, Clone)]
pub struct DeployConfig {
    /// Does not submit a transaction, but instead estimates the gas required
    /// to complete the operation.
    #[arg(long)]
    estimate_gas_only: bool,
    /// By default, submits two transactions to deploy and activate the program to Arbitrum.
    /// Otherwise, a user could choose to split up the deploy and activate steps into individual transactions.
    #[arg(long, value_enum)]
    mode: Option<DeployMode>,
    /// The endpoint of the L2 node to connect to.
    #[arg(short, long, default_value = "http://localhost:8545")]
    endpoint: String,
    /// Wallet source to use with the cargo stylus plugin.
    #[command(flatten)]
    keystore_opts: KeystoreOpts,
    /// Privkey source to use with the cargo stylus plugin.
    #[arg(long)]
    private_key_path: Option<String>,
    /// If only activating an already-deployed, onchain program, the address of the program to send an activation tx for.
    #[arg(long)]
    activate_program_address: Option<H160>,
    /// If desired, it loads a WASM file from a specified path. If not provided, it will try to find
    /// a WASM file under the current working directory's Rust target release directory and use its
    /// contents for the deploy command.
    #[arg(long)]
    wasm_file_path: Option<String>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum DeployMode {
    DeployOnly,
    ActivateOnly,
}

#[derive(Clone, Debug, Args)]
#[group(multiple = true)]
pub struct KeystoreOpts {
    #[arg(long)]
    keystore_path: Option<String>,
    #[arg(long)]
    keystore_password_path: Option<String>,
}

#[tokio::main]
async fn main() -> eyre::Result<(), String> {
    let CargoCli::Stylus(args) = CargoCli::parse();

    match args.command {
        StylusSubcommands::Check(cfg) => {
            if let Err(e) = check::run_checks(cfg).await {
                println!("Stylus checks failed: {:?}", e.red());
            };
        }
        StylusSubcommands::Deploy(cfg) => {
            if let Err(e) = deploy::deploy(cfg).await {
                println!("Deploy / activation command failed: {:?}", e.red());
            };
        }
    }
    Ok(())
}
