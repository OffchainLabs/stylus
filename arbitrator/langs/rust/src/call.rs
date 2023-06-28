// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use crate::{Bytes20, Bytes32};

#[derive(Default)]
pub struct Call {
    kind: Kind,
    value: Bytes32,
    gas: Option<u64>,
    offset: u64,
    len: Option<u64>,
}

#[derive(PartialEq)]
pub enum Kind {
    Basic,
    Delegate,
    Static,
}

impl Default for Kind {
    fn default() -> Self {
        Kind::Basic
    }
}

#[link(wasm_import_module = "forward")]
extern "C" {
    fn call_contract(
        contract: *const u8,
        calldata: *const u8,
        calldata_len: usize,
        value: *const u8,
        ink: u64,
        return_data_len: *mut usize,
    ) -> u8;

    fn delegate_call_contract(
        contract: *const u8,
        calldata: *const u8,
        calldata_len: usize,
        ink: u64,
        return_data_len: *mut usize,
    ) -> u8;

    fn static_call_contract(
        contract: *const u8,
        calldata: *const u8,
        calldata_len: usize,
        ink: u64,
        return_data_len: *mut usize,
    ) -> u8;

    /// A noop when there's never been a call
    fn read_return_data(dest: *mut u8, offset: u64, size: u64);
}

impl Call {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_delegate() -> Self {
        Self {
            kind: Kind::Delegate,
            ..Default::default()
        }
    }

    pub fn new_static() -> Self {
        Self {
            kind: Kind::Static,
            ..Default::default()
        }
    }

    pub fn value(&mut self, value: Bytes32) -> &mut Self {
        self.value = value;
        self
    }

    pub fn gas(&mut self, gas: u64) -> &mut Self {
        self.gas = Some(gas);
        self
    }

    pub fn subset(&mut self, offset: u64, len: u64) -> &mut Self {
        self.offset = offset;
        self.len = Some(len);
        self
    }

    pub fn skip_return_data(&mut self) -> &mut Self {
        self.len = Some(0);
        self
    }

    pub fn call(&self, contract: Bytes20, calldata: &[u8]) -> Result<Vec<u8>, Vec<u8>> {
        let mut outs_len = 0;
        if self.value != Bytes32::default() && self.kind != Kind::Basic {
            return Err("unexpected value".into());
        }

        let gas = self.gas.unwrap_or(u64::MAX); // will be clamped by 63/64 rule
        let status = match self.kind {
            Kind::Basic => unsafe {
                call_contract(
                    contract.ptr(),
                    calldata.as_ptr(),
                    calldata.len(),
                    self.value.ptr(),
                    gas,
                    &mut outs_len,
                )
            },
            Kind::Delegate => unsafe {
                delegate_call_contract(
                    contract.ptr(),
                    calldata.as_ptr(),
                    calldata.len(),
                    gas,
                    &mut outs_len,
                )
            },
            Kind::Static => unsafe {
                static_call_contract(
                    contract.ptr(),
                    calldata.as_ptr(),
                    calldata.len(),
                    gas,
                    &mut outs_len,
                )
            },
        };

        let len = self.len.unwrap_or(outs_len as u64);
        let outs = if len == 0 {
            vec![]
        } else {
            unsafe {
                let mut outs = Vec::with_capacity(len as usize);
                read_return_data(outs.as_mut_ptr(), self.offset, len);
                outs.set_len(len as usize);
                outs
            }
        };
        match status {
            0 => Ok(outs),
            _ => Err(outs),
        }
    }
}
