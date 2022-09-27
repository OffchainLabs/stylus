// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use arbitrary::Unstructured;
use wasm_smith::{Config, Module};

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
    fn min_funcs(&self) -> usize {
        self.min_funcs // upstream bug ignores this for small slices
    }
    fn max_funcs(&self) -> usize {
        100
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
    fn max_instructions(&self) -> usize {
        256
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
