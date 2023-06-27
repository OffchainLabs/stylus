// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use eyre::Result;
use humantime::Duration;
use std::path::PathBuf;
use structopt::StructOpt;

mod analyze;
mod check;
mod evm_api;
mod model;
mod record;
mod util;
mod wasm;

#[derive(StructOpt)]
#[structopt(name = "pricer")]
struct Opts {
    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(StructOpt)]
enum Command {
    #[structopt(name = "record")]
    Record {
        #[structopt(short, long)]
        path: PathBuf,
        #[structopt(short, long)]
        duration: Duration,
    },

    #[structopt(name = "model")]
    Model {
        #[structopt(short, long)]
        path: PathBuf,
        #[structopt(short, long)]
        output: PathBuf,
    },

    #[structopt(name = "analyze")]
    Analyze {
        #[structopt(short, long)]
        model: Option<PathBuf>,
        #[structopt(short, long)]
        trials: Option<PathBuf>,
    },

    #[structopt(name = "check")]
    Check {
        #[structopt(short, long)]
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    let opts = Opts::from_args();

    match opts.cmd {
        Command::Record { path, duration } => record::record(&path, *duration),
        Command::Model { path, output } => model::model(&path, &output),
        Command::Analyze { model, trials } => analyze::analyze(model, trials),
        Command::Check { path } => check::check(&path),
    }
}
