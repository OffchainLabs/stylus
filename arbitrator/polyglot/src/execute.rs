// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use crate::machine::{Escape, WasmEnvArc};
use eyre::{bail, Result};
use prover::{
    machine::MachineStatus,
    middlewares::{
        depth::DepthCheckedMachine,
        meter::{MachineMeter, MeteredMachine},
    },
    Machine,
};
use std::fmt::Display;
use wasmer::Instance;

#[derive(Debug)]
pub enum ExecOutcome {
    NoStart,
    Success(Vec<u8>),
    Revert(Vec<u8>),
    Failure(String),
    OutOfGas,
    OutOfStack,
    StackOverflow,
}

impl Display for ExecOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ExecOutcome::*;
        match self {
            NoStart => write!(f, "no start"),
            Success(output) => write!(f, "success {}", hex::encode(output)),
            Revert(output) => write!(f, "revert {}", hex::encode(output)),
            Failure(error) => write!(f, "failure: {}", error),
            OutOfGas => write!(f, "out of gas"),
            OutOfStack => write!(f, "out of stack"),
            StackOverflow => write!(f, "stack overflow"),
        }
    }
}

impl PartialEq for ExecOutcome {
    fn eq(&self, other: &Self) -> bool {
        use ExecOutcome::*;
        match self {
            NoStart => matches!(other, NoStart),
            Failure(_) => matches!(other, Failure(_)),
            OutOfGas => matches!(other, OutOfGas),
            OutOfStack => matches!(other, OutOfStack),
            StackOverflow => matches!(other, StackOverflow),
            Success(output) => match other {
                Success(other) => output == other,
                _ => false,
            },
            Revert(output) => match other {
                Revert(other) => output == other,
                _ => false,
            },
        }
    }
}

pub trait ExecPolyglot {
    fn run_start(&mut self) -> ExecOutcome;
    fn run_main(&self, env: WasmEnvArc) -> Result<ExecOutcome>;
}

impl ExecPolyglot for Instance {
    fn run_start(&mut self) -> ExecOutcome {
        let start = match self.exports.get_function("polyglot_moved_start").ok() {
            Some(start) => start.native::<(), ()>().unwrap(),
            None => return ExecOutcome::NoStart,
        };

        match start.call() {
            Ok(_) => ExecOutcome::Success(vec![]),
            Err(error) => {
                if let MachineMeter::Exhausted = self.gas_left() {
                    return ExecOutcome::OutOfGas;
                }
                if self.stack_space_left() == 0 {
                    return ExecOutcome::OutOfStack;
                }

                let error = error.to_string();
                match error.contains("RuntimeError: call stack exhausted") {
                    true => ExecOutcome::StackOverflow,
                    false => ExecOutcome::Failure(error),
                }
            }
        }
    }

    fn run_main(&self, env: WasmEnvArc) -> Result<ExecOutcome> {
        let main = self.exports.get_function("arbitrum_main")?;
        let main = main.native::<i32, ()>()?;
        let args = env.lock().args.len() as i32;

        let escape = match main.call(args) {
            Ok(()) => bail!("program failed to canonically exit"),
            Err(outcome) => Escape::from(outcome),
        };

        if self.stack_space_left() == 0 {
            return Ok(ExecOutcome::OutOfStack);
        }
        if self.gas_left() == MachineMeter::Exhausted {
            return Ok(ExecOutcome::OutOfGas);
        };
        let status = match escape {
            Escape::OutOfGas => return Ok(ExecOutcome::OutOfGas),
            Escape::Failure(err) => return Ok(ExecOutcome::Failure(err)),
            Escape::HostIO(err) => return Ok(ExecOutcome::Failure(err)),
            Escape::Exit(status) => status,
        };

        let output = env.lock().output.to_vec();
        Ok(match status {
            0 => ExecOutcome::Success(output),
            _ => ExecOutcome::Revert(output),
        })
    }
}

impl ExecPolyglot for Machine {
    fn run_start(&mut self) -> ExecOutcome {
        if self.get_function("polyglot_moved_start").is_none() {
            return ExecOutcome::NoStart;
        }

        let call = self.call_function("polyglot_moved_start", &vec![]).unwrap();

        match call {
            Ok(_) => ExecOutcome::Success(vec![]),
            Err(error) => {
                if let MachineMeter::Exhausted = self.gas_left() {
                    return ExecOutcome::OutOfGas;
                }
                if self.stack_space_left() == 0 {
                    return ExecOutcome::OutOfStack;
                }

                match error {
                    MachineStatus::Running => ExecOutcome::StackOverflow,
                    MachineStatus::Errored => ExecOutcome::Failure("error".to_owned()),
                    MachineStatus::Finished => panic!("machine finished unsuccessfully"),
                    MachineStatus::TooFar => panic!("machine reached the too-far state"),
                }
            }
        }
    }

    fn run_main(&self, _env: WasmEnvArc) -> Result<ExecOutcome> {
        todo!()
    }
}
