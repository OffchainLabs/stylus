// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

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
    Success,
    Failure(String),
    OutOfGas,
    OutOfStack,
    StackOverflow,
}

impl Display for ExecOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ExecOutcome::*;
        match self {
            Success => write!(f, "success"),
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
            Success => matches!(other, Success),
            Failure(_) => matches!(other, Failure(_)),
            OutOfGas => matches!(other, OutOfGas),
            OutOfStack => matches!(other, OutOfStack),
            StackOverflow => matches!(other, StackOverflow),
        }
    }
}

pub trait ExecPolyglot {
    fn execute(&mut self) -> ExecOutcome;
}

impl ExecPolyglot for Instance {
    fn execute(&mut self) -> ExecOutcome {
        let start = match self.exports.get_function("polyglot_moved_start").ok() {
            Some(start) => start.native::<(), ()>().unwrap(),
            None => return ExecOutcome::Success,
        };

        match start.call() {
            Ok(_) => ExecOutcome::Success,
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
}

impl ExecPolyglot for Machine {
    fn execute(&mut self) -> ExecOutcome {
        if self.get_function("polyglot_moved_start").is_none() {
            return ExecOutcome::Success;
        }

        let call = self.call_function("polyglot_moved_start", &vec![]).unwrap();

        match call {
            Ok(_) => ExecOutcome::Success,
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
}
