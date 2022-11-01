// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use crate::{machine, machine::WasmEnvArc, ExecOutcome, ExecPolyglot};
use prover::programs::{meter::MeteredMachine, PolyglotConfig};

const POLYGLOT_SUCCESS: usize = 0;
const POLYGLOT_FAILURE: usize = 1;

#[no_mangle]
pub unsafe extern "C" fn polyglot_compile(
    wasm: *const u8,
    len: usize,
    out: *mut *const u8,
    out_len: *mut usize,
    out_cap: *mut usize,
) -> usize {
    let wasm = std::slice::from_raw_parts(wasm, len);

    macro_rules! finish {
        ($bytes:expr, $code:expr) => {{
            *out_len = $bytes.len();
            *out_cap = $bytes.capacity();
            *out = $bytes.as_ptr();
            std::mem::forget($bytes);
            return $code;
        }};
    }

    let config = PolyglotConfig::default();
    match machine::instrument(&wasm, &config) {
        Ok((module, _)) => finish!(module, POLYGLOT_SUCCESS),
        Err(error) => {
            let error = error.to_string();
            finish!(error, POLYGLOT_FAILURE);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyglot_call(
    module: *const u8,
    module_len: usize,
    input: *const u8,
    input_len: usize,
    output: *mut *const u8,
    output_len: *mut usize,
    output_cap: *mut usize,
    gas: *mut u64,
    gas_price: u64,
) -> usize {
    let module = std::slice::from_raw_parts(module, module_len);
    let input = std::slice::from_raw_parts(input, input_len);
    let env = WasmEnvArc::new(input, gas_price);

    macro_rules! finish {
        ($bytes:expr, $code:expr) => {{
            *output_len = $bytes.len();
            *output_cap = $bytes.capacity();
            *output = $bytes.as_ptr();
            std::mem::forget($bytes);
            return $code;
        }};
    }
    macro_rules! error {
        ($report:expr) => {{
            let error = $report.to_string();
            finish!(error, POLYGLOT_FAILURE);
        }};
    }

    let config = PolyglotConfig::default();
    let store = match config.store() {
        Ok(store) => store,
        Err(error) => error!(error),
    };
    let mut instance = match machine::create(&module, &store, env.clone()) {
        Ok(instance) => instance,
        Err(error) => error!(error),
    };
    instance.set_gas(*gas);

    let outcome = match instance.run_main(env.clone()) {
        Ok(outcome) => outcome,
        Err(error) => error!(error),
    };
    *gas = instance.gas_left().into();

    match outcome {
        ExecOutcome::Success(output) => finish!(output, POLYGLOT_SUCCESS),
        ExecOutcome::Revert(output) => finish!(output, POLYGLOT_FAILURE),
        failure => error!(failure),
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyglot_free(data: *mut u8, len: usize, cap: usize) {
    let vec = Vec::from_raw_parts(data, len, cap);
    std::mem::drop(vec)
}
