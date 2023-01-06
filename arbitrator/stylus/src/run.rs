// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use std::fmt::Display;

use crate::{env::Escape, stylus::NativeInstance};
use eyre::{ensure, ErrReport, Result};
use prover::machine::Machine;
use prover::programs::prelude::*;

pub enum UserOutcome {
    Success(Vec<u8>),
    Revert(Vec<u8>),
    OutOfGas,
    OutOfStack,
}

impl UserOutcome {
    fn revert(error: ErrReport) -> Self {
        let data = format!("{:?}", error);
        Self::Revert(data.into_bytes())
    }
}

impl Display for UserOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use UserOutcome::*;
        match self {
            Success(data) => write!(f, "success {}", hex::encode(data)),
            Revert(data) => write!(f, "revert {}", hex::encode(data)),
            OutOfGas => write!(f, "out of gas"),
            OutOfStack => write!(f, "out of stack"),
        }
    }
}

pub trait RunProgram {
    fn run_main(&mut self, args: &[u8], config: &StylusConfig) -> Result<UserOutcome>;
}

impl RunProgram for Machine {
    fn run_main(&mut self, args: &[u8], config: &StylusConfig) -> Result<UserOutcome> {
        let pricing = &config.pricing;

        macro_rules! call {
            ($module:expr, $func:expr, $args:expr) => {
                call!($module, $func, $args, |error| Err(error))
            };
            ($module:expr, $func:expr, $args:expr, $error:expr) => {{
                match self.call_function($module, $func, $args) {
                    Ok(value) => value[0].try_into().unwrap(),
                    Err(error) => return $error(error),
                }
            }};
        }

        // push the args
        let args_len = (args.len() as u32).into();
        let push_vec = vec![
            args_len,
            pricing.wasm_gas_price.into(),
            pricing.hostio_cost.into(),
        ];
        let args_ptr = call!("user_host", "push_program", push_vec);
        let user_host = self.find_module("user_host")?;
        self.write_memory(user_host, args_ptr, args)?;

        let status: u32 = call!("user", "arbitrum_main", vec![args_len], |error| {
            if self.gas_left() == MachineMeter::Exhausted {
                return Ok(UserOutcome::OutOfGas);
            }
            if self.stack_left() == 0 {
                return Ok(UserOutcome::OutOfStack);
            }
            Err(error)
        });

        let outs_len = call!("user_host", "get_output_len", vec![]);
        let outs_ptr = call!("user_host", "get_output_ptr", vec![]);
        let outs = self.read_memory(user_host, outs_len, outs_ptr)?.to_vec();

        let num_progs: u32 = call!("user_host", "pop_program", vec![]);
        ensure!(num_progs == 0, "dirty user_host");

        Ok(match status {
            0 => UserOutcome::Success(outs),
            _ => UserOutcome::Revert(outs),
        })
    }
}

impl RunProgram for NativeInstance {
    fn run_main(&mut self, args: &[u8], _config: &StylusConfig) -> Result<UserOutcome> {
        let store = &mut self.store;
        let exports = &self.instance.exports;
        let main = exports.get_typed_function::<u32, u32>(store, "arbitrum_main")?;
        let status = match main.call(store, args.len() as u32) {
            Ok(status) => status,
            Err(outcome) => {
                let escape = outcome.downcast()?;

                if self.stack_left() == 0 {
                    return Ok(UserOutcome::OutOfStack);
                }
                if self.gas_left() == MachineMeter::Exhausted {
                    return Ok(UserOutcome::OutOfGas);
                }

                return Ok(match escape {
                    Escape::OutOfGas => UserOutcome::OutOfGas,
                    Escape::Memory(error) => UserOutcome::revert(error.into()),
                    Escape::Internal(error) => UserOutcome::revert(error),
                });
            }
        };

        let outs = self.env().outs.clone();
        Ok(match status {
            0 => UserOutcome::Success(outs),
            _ => UserOutcome::Revert(outs),
        })
    }
}