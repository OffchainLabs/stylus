// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use std::convert::TryInto;
use std::fmt::Display;

use eyre::{bail, Result};

use super::{
    depth::DepthCheckedMachine,
    meter::{MachineMeter, MeteredMachine},
};
use crate::{Machine, MachineStatus, Value};

pub enum ExecOutcome {
    Success(Vec<u8>),
    Revert(Vec<u8>),
    Failure(String),
    OutOfGas,
    OutOfStack,
    DivergingFailure,
}

impl Display for ExecOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ExecOutcome::*;
        match self {
            Success(output) => write!(f, "success {}", hex::encode(output)),
            Revert(output) => write!(f, "revert {}", hex::encode(output)),
            Failure(error) => write!(f, "failure: {}", error),
            OutOfGas => write!(f, "out of gas"),
            OutOfStack => write!(f, "out of stack"),
            DivergingFailure => write!(f, "diverging failure"),
        }
    }
}

pub trait ExecProgram {
    fn run_main(&mut self, args: Vec<u8>) -> Result<ExecOutcome>;
}

impl ExecProgram for Machine {
    fn run_main(&mut self, args: Vec<u8>) -> Result<ExecOutcome> {
        let args_len = Value::from(args.len() as u32);
        let args_ptr = match self.call_function("poly_host", "allocate_args", &vec![args_len])? {
            Ok(ptr) => ptr[0].try_into().unwrap(),
            Err(status) => bail!("failed to allocate args: {}", status),
        };
        self.write_memory("poly_host", args_ptr, &args)?;

        let status: u32 = match self.call_function("user", "arbitrum_main", &vec![args_len])? {
            Ok(value) => value[0].try_into().unwrap(),
            Err(status) => {
                if self.gas_left() == MachineMeter::Exhausted {
                    return Ok(ExecOutcome::OutOfGas);
                }
                if self.stack_space_left() == 0 {
                    return Ok(ExecOutcome::OutOfStack);
                }

                let error = match status {
                    MachineStatus::Running => "machine not done",
                    MachineStatus::Errored => "machine errored",
                    MachineStatus::Finished => "machine halted",
                    MachineStatus::TooFar => "machine reached the too-far state",
                };
                return Ok(ExecOutcome::Failure(error.into()));
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
