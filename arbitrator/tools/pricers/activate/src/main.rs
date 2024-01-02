// Copyright 2022-2024, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use activate::Trial;
use eyre::Result;
use std::path::PathBuf;
use structopt::StructOpt;

mod check;
mod csv;
mod verify;

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
        count: u64,
    },

    #[structopt(name = "check")]
    Check {
        #[structopt(short, long)]
        path: PathBuf,
    },

    #[structopt(name = "csv")]
    Csv {
        #[structopt(short, long)]
        path: PathBuf,
        #[structopt(short, long)]
        field: String,
    },

    #[structopt(name = "verify")]
    Verify {
        #[structopt(short, long)]
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    let opts = Opts::from_args();
    match opts.cmd {
        Command::Check { path } => check::check(path),
        Command::Csv { path, field } => csv::csv(path, field),
        Command::Record { path, count } => record::record(path, count),
        Command::Verify { path } => verify::verify(&path),
    }
}
