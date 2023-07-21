use clap::{Parser, Subcommand};

mod check;
mod constants;
mod deploy;

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
    /// Instrument a Rust project using Stylus, optionally outputting the brotli-compressed,
    /// compiled WASM code to deploy on-chain. This command runs compiled WASM code through
    /// Stylus instrumentation checks and reports any failures. Allows for disabling specific .
    /// checks via the `--disabled-checks` flag.
    #[command(alias = "c")]
    Check {
        disabled_checks: Option<Vec<String>>,
        output_file: Option<String>,
    },
    /// Instruments a Rust project using Stylus and by outputting its brotli-compressed WASM code.
    /// Then, it submits a single, multicall transaction that both deploys the WASM
    /// program to an address and triggers a compilation onchain by default. This transaction is atomic,
    /// and will revert if either the program creation or onchain compilation step fails.
    /// Developers can choose to split up the deploy and compile steps via this command as desired.
    #[command(alias = "d")]
    Deploy {
        /// Does not submit a transaction, but instead estimates the gas required
        /// to complete the operation.
        #[arg(long, default_value = "false")]
        estimate_gas: bool,
        /// Disables the onchain compilation step of the deploy process.
        /// This flag is useful for developers who want to split up the deploy and compile steps.
        #[arg(long, default_value = "false")]
        only_deploy: bool,
        /// Disables the onchain deploy step of the deploy process.
        /// This flag is useful for developers who want to split up the deploy and compile steps.
        #[arg(long, default_value = "false")]
        only_compile: bool,
        /// The endpoint of the L2 node to connect to.
        #[arg(short, long, default_value = "http://localhost:8545")]
        endpoint: String,
        /// Address of a multicall Stylus program on L2 to use for the atomic, onchain deploy+compile
        /// operation. If not provided, address <INSERT_ADDRESS_HERE> will be used.
        #[arg(long)]
        // TODO: Use an alloy primitive address type for this.
        multicall_program_addr: Option<String>,
    },
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Check { disabled_checks, .. } => {
            let disabled = disabled_checks.as_ref().map(|f| {
                f.into_iter()
                    .map(|s| s.as_str().into())
                    .collect::<Vec<check::StylusCheck>>()
            });
            check::run_checks(disabled)
        }
        Commands::Deploy { .. } => {
            todo!();
        }
    }
}
