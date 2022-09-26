// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

pub mod execute;
pub mod machine;

pub use execute::{ExecOutcome, ExecPolyglot};

#[cfg(test)]
mod test;
