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
    Machine, Value,
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
    FatalError(String),
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
            FatalError(error) => write!(f, "fatal error: {}", error),
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
            FatalError(_) => matches!(other, FatalError(_)),
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
    fn run_main(&mut self, env: WasmEnvArc) -> Result<ExecOutcome>;
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

    fn run_main(&mut self, env: WasmEnvArc) -> Result<ExecOutcome> {
        let main = self.exports.get_function("user__arbitrum_main")?;
        let main = main.native::<i32, i32>()?;
        let args = env.lock().args.len() as i32;

        let status = match main.call(args) {
            Ok(status) => status,
            Err(outcome) => {
                let escape = Escape::from(outcome);

                if self.stack_space_left() == 0 {
                    return Ok(ExecOutcome::OutOfStack);
                }
                if self.gas_left() == MachineMeter::Exhausted {
                    return Ok(ExecOutcome::OutOfGas);
                };

                return Ok(match escape {
                    Escape::OutOfGas => ExecOutcome::OutOfGas,
                    Escape::Failure(err) => ExecOutcome::Failure(err),
                    Escape::HostIO(err) => ExecOutcome::Failure(err),
                    Escape::Exit(code) => ExecOutcome::Failure(format!("exited with code {code}")),
                });
            }
        };

        let outs = env.lock().outs.to_vec();
        Ok(match status {
            0 => ExecOutcome::Success(outs),
            _ => ExecOutcome::Revert(outs),
        })
    }
}

impl ExecPolyglot for Machine {
    fn run_start(&mut self) -> ExecOutcome {
        if self.get_function("user", "polyglot_moved_start").is_err() {
            return ExecOutcome::NoStart;
        }

        let call = match self.call_function("user", "polyglot_moved_start", &vec![]) {
            Ok(call) => call,
            Err(error) => return ExecOutcome::FatalError(error.to_string()),
        };

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
                    MachineStatus::Running => ExecOutcome::FatalError("not done".into()),
                    MachineStatus::Errored => ExecOutcome::Failure("error".into()),
                    MachineStatus::Finished => panic!("machine finished unsuccessfully"),
                    MachineStatus::TooFar => panic!("machine reached the too-far state"),
                }
            }
        }
    }

    fn run_main(&mut self, env: WasmEnvArc) -> Result<ExecOutcome> {
        let args_len = Value::from(env.lock().args.len() as u32);
        let args_ptr = match self.call_function("poly_host", "allocate_args", &vec![args_len])? {
            Ok(ptr) => ptr[0],
            Err(status) => bail!("failed to allocate memory: {}", status),
        };
        self.write_memory("poly_host", args_ptr.try_into().unwrap(), &env.lock().args)?;

        let status: u32 = match self.call_function("user", "arbitrum_main", &vec![args_len])? {
            Ok(value) => value[0].try_into().unwrap(),
            Err(status) => {
                if self.gas_left() == MachineMeter::Exhausted {
                    return Ok(ExecOutcome::OutOfGas);
                }
                if self.stack_space_left() == 0 {
                    return Ok(ExecOutcome::OutOfStack);
                }

                use MachineStatus::*;
                return Ok(match status {
                    Running => ExecOutcome::FatalError("not done".into()),
                    Errored => ExecOutcome::Failure("error".into()),
                    Finished => panic!("machine finished unsuccessfully"),
                    TooFar => panic!("machine reached the too-far state"),
                });
            }
        };

        let outs_len = match self.call_function("poly_host", "read_output_len", &vec![])? {
            Ok(output) => output[0].try_into().unwrap(),
            Err(status) => bail!("failed to read output data length: {}", status),
        };
        let outs_ptr = match self.call_function("poly_host", "read_output_ptr", &vec![])? {
            Ok(output) => output[0].try_into().unwrap(),
            Err(status) => bail!("failed to read output data: {}", status),
        };

        let outs = self.read_memory("poly_host", outs_len, outs_ptr)?.to_vec();
        Ok(match status {
            0 => ExecOutcome::Success(outs),
            _ => ExecOutcome::Revert(outs),
        })
    }
}
