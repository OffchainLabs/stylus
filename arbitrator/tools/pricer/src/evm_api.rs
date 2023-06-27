// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use arbutil::{
    evm::{api::EvmApi, user::UserOutcomeKind},
    Bytes20, Bytes32,
};
use eyre::Result;

pub(crate) struct PanicApi;

impl EvmApi for PanicApi {
    fn get_bytes32(&mut self, _key: Bytes32) -> (Bytes32, u64) {
        unimplemented!()
    }

    fn set_bytes32(&mut self, _key: Bytes32, _value: Bytes32) -> Result<u64> {
        unimplemented!()
    }

    fn contract_call(
        &mut self,
        _contract: Bytes20,
        _calldata: Vec<u8>,
        _gas: u64,
        _value: Bytes32,
    ) -> (u32, u64, UserOutcomeKind) {
        unimplemented!()
    }

    fn delegate_call(
        &mut self,
        _contract: Bytes20,
        _calldata: Vec<u8>,
        _gas: u64,
    ) -> (u32, u64, UserOutcomeKind) {
        unimplemented!()
    }

    fn static_call(
        &mut self,
        _contract: Bytes20,
        _calldata: Vec<u8>,
        _gas: u64,
    ) -> (u32, u64, UserOutcomeKind) {
        unimplemented!()
    }

    fn create1(
        &mut self,
        _code: Vec<u8>,
        _endowment: Bytes32,
        _gas: u64,
    ) -> (eyre::Result<Bytes20>, u32, u64) {
        unimplemented!()
    }

    fn create2(
        &mut self,
        _code: Vec<u8>,
        _endowment: Bytes32,
        _salt: Bytes32,
        _gas: u64,
    ) -> (eyre::Result<Bytes20>, u32, u64) {
        unimplemented!()
    }

    fn get_return_data(&mut self) -> Vec<u8> {
        unimplemented!()
    }

    fn emit_log(&mut self, _data: Vec<u8>, _topics: u32) -> Result<()> {
        unimplemented!()
    }

    fn account_balance(&mut self, _address: Bytes20) -> (Bytes32, u64) {
        unimplemented!()
    }

    fn account_codehash(&mut self, _address: Bytes20) -> (Bytes32, u64) {
        unimplemented!()
    }

    fn evm_blockhash(&mut self, _number: Bytes32) -> Bytes32 {
        unimplemented!()
    }

    fn add_pages(&mut self, _pages: u16) -> u64 {
        u64::MAX
    }
}
