// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use wasm_smith::{Config, Module};

use std::borrow::Cow;
use libfuzzer_sys::arbitrary::Unstructured;
use eyre::{bail, Result};
use wasmparser::{Validator, WasmFeatures};

#[derive(Debug)]
pub struct WasmConfig {
    min_funcs: usize,
}

impl WasmConfig {
    fn new(min_funcs: usize) -> Self {
        Self { min_funcs }
    }
}

impl Config for WasmConfig {
    fn available_imports(&self) -> Option<Cow<'_, [u8]>> {
        Some(wasmer::wat2wasm(r#"(module)"#.as_bytes()).unwrap())
    }
    fn min_funcs(&self) -> usize {
        self.min_funcs // upstream bug ignores this for small slices
    }
    fn max_funcs(&self) -> usize {
        20
    }
    fn max_instructions(&self) -> usize {
        256
    }
    fn max_memory_pages(&self, _is_64: bool) -> u64 {
        33 // a little over 2 MB
    }
    fn memory_offset_choices(&self) -> (u32, u32, u32) {
        (95, 4, 1) // out-of-bounds 5% of the time
    }
    fn multi_value_enabled(&self) -> bool {
        false // research why Singlepass doesn't have this on by default before enabling
    }
    fn allow_start_export(&self) -> bool {
        true
    }
    fn require_start_export(&self) -> bool {
        true
    }
    fn memory64_enabled(&self) -> bool {
        false
    }
    fn canonicalize_nans(&self) -> bool {
        false
    }
    fn threads_enabled(&self) -> bool {
        false
    }
    fn min_memories(&self) -> u32 {
        1
    }
    fn max_memories(&self) -> usize {
        1
    }
    fn export_everything(&self) -> bool {
        true
    }
}

pub fn random(noise: &[u8], min_funcs: usize) -> Vec<u8> {
    let mut input = Unstructured::new(noise);
    let module = Module::new(WasmConfig::new(min_funcs), &mut input).unwrap();
    module.to_bytes()
}

pub fn validate(input: &[u8]) -> bool {
    let features = WasmFeatures {
        mutable_global: true,
        saturating_float_to_int: true,
        sign_extension: true,
        reference_types: false,
        multi_value: true,
        bulk_memory: false,
        module_linking: false,
        simd: false,
        relaxed_simd: false,
        threads: false,
        tail_call: false,
        deterministic_only: false,
        multi_memory: false,
        exceptions: false,
        memory64: false,
        extended_const: false,
    };
    let mut validator = Validator::new();
    validator.wasm_features(features);

    validator.validate_all(input).is_ok()
}
