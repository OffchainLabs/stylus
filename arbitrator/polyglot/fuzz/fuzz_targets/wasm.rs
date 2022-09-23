// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

use arbitrary::{Arbitrary, Unstructured};
use wasm_smith::{Config, ConfiguredModule};

use std::borrow::Cow;

#[derive(Arbitrary, Debug)]
struct WasmConfig {}

impl Config for WasmConfig {
    fn available_imports(&self) -> Option<Cow<'_, [u8]>> {
        Some(wasmer::wat2wasm(r#"(module)"#.as_bytes()).unwrap())
    }
    fn canonicalize_nans(&self) -> bool {
        false
    }
    fn max_memory_pages(&self, _is_64: bool) -> u64 {
        17 // a little over 1 MB
    }
    fn memory64_enabled(&self) -> bool {
        false
    }
    fn memory_offset_choices(&self) -> (u32, u32, u32) {
        // ensure all memory accesses are in bounds
        (95, 4, 1)
    }
    fn multi_value_enabled(&self) -> bool {
        // research why Singlepass doesn't have this on by default before enabling
        false
    }
    fn threads_enabled(&self) -> bool {
        false
    }
}

pub fn random(noise: &[u8]) -> Vec<u8> {
    let mut input = Unstructured::new(noise);
    let module = ConfiguredModule::<WasmConfig>::arbitrary(&mut input)
        .unwrap()
        .module;
    module.to_bytes()
}
