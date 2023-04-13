// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use arbitrary::Unstructured;
use eyre::Result;
use wasm_smith::{Config, Module};
use wasmer::wasmparser::{Validator, WasmFeatures};

use std::borrow::Cow;

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
    fn canonicalize_nans(&self) -> bool {
        false
    }
    fn max_elements(&self) -> usize {
        0
    }
    fn max_element_segments(&self) -> usize {
        0
    }
    fn max_components(&self) -> usize {
        0
    }
    fn max_data_segments(&self) -> usize {
        0
    }
    fn max_tags(&self) -> usize {
        0
    }
    fn min_funcs(&self) -> usize {
        self.min_funcs // upstream bug ignores this for small slices
    }
    fn max_funcs(&self) -> usize {
        2
    }
    fn max_instructions(&self) -> usize {
        1_000
    }
    fn min_exports(&self) -> usize {
        1
    }
    fn memory_name(&self) -> Option<String> {
        Some("memory".into())
    }
    fn min_memories(&self) -> u32 {
        1
    }
    fn max_memory_pages(&self, _is_64: bool) -> u64 {
        17 // a little over 1 MB
    }
    fn memory64_enabled(&self) -> bool {
        false
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
    fn threads_enabled(&self) -> bool {
        false
    }
}

pub fn random(noise: &[u8], min_funcs: usize) -> Vec<u8> {
    let mut input = Unstructured::new(noise);
    let module = Module::new(WasmConfig::new(min_funcs), &mut input).unwrap();
    module.to_bytes()
}

pub fn validate(wasm: &[u8]) -> Result<()> {
    let features = WasmFeatures {
        mutable_global: true,
        saturating_float_to_int: true,
        sign_extension: true,
        reference_types: false,
        multi_value: false,
        bulk_memory: false, // not all ops supported yet
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
    validator.validate_all(wasm)?;

    Ok(())
}
