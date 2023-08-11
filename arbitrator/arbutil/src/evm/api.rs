// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use crate::{
    evm::{user::UserOutcomeKind, Opcode},
    Bytes20, Bytes32,
};
use eyre::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EvmApiStatus {
    Success,
    Failure,
}

impl From<EvmApiStatus> for UserOutcomeKind {
    fn from(value: EvmApiStatus) -> Self {
        match value {
            EvmApiStatus::Success => UserOutcomeKind::Success,
            EvmApiStatus::Failure => UserOutcomeKind::Revert,
        }
    }
}

impl From<u8> for EvmApiStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Success,
            _ => Self::Failure,
        }
    }
}

#[repr(usize)]
pub enum EvmApiMethod {
    GetBytes32,
    SetBytes32,
    ContractCall,
    DelegateCall,
    StaticCall,
    Create1,
    Create2,
    GetReturnData,
    EmitLog,
    ReportHostio,
    ReportHostioAdvanced,
    AccountBalance,
    AccountCodeHash,
    AddPages,
}

pub trait EvmApi: Send + 'static {
    /// Reads the 32-byte value in the EVM state trie at offset `key`.
    /// Returns the value and the access cost in gas.
    /// Analogous to `vm.SLOAD`.
    fn get_bytes32(&mut self, key: Bytes32) -> (Bytes32, u64);

    /// Stores the given value at the given key in the EVM state trie.
    /// Returns the access cost on success.
    /// Analogous to `vm.SSTORE`.
    fn set_bytes32(&mut self, key: Bytes32, value: Bytes32) -> Result<u64>;

    /// Calls the contract at the given address.
    /// Returns the EVM return data's length, the gas cost, and whether the call succeeded.
    /// Analogous to `vm.CALL`.
    fn contract_call(
        &mut self,
        contract: Bytes20,
        calldata: Vec<u8>,
        gas: u64,
        value: Bytes32,
    ) -> (u32, u64, UserOutcomeKind);

    /// Delegate-calls the contract at the given address.
    /// Returns the EVM return data's length, the gas cost, and whether the call succeeded.
    /// Analogous to `vm.DELEGATECALL`.
    fn delegate_call(
        &mut self,
        contract: Bytes20,
        calldata: Vec<u8>,
        gas: u64,
    ) -> (u32, u64, UserOutcomeKind);

    /// Static-calls the contract at the given address.
    /// Returns the EVM return data's length, the gas cost, and whether the call succeeded.
    /// Analogous to `vm.STATICCALL`.
    fn static_call(
        &mut self,
        contract: Bytes20,
        calldata: Vec<u8>,
        gas: u64,
    ) -> (u32, u64, UserOutcomeKind);

    /// Deploys a new contract using the init code provided.
    /// Returns the new contract's address on success, or the error reason on failure.
    /// In both cases the EVM return data's length and the overall gas cost are returned too.
    /// Analogous to `vm.CREATE`.
    fn create1(
        &mut self,
        code: Vec<u8>,
        endowment: Bytes32,
        gas: u64,
    ) -> (Result<Bytes20>, u32, u64);

    /// Deploys a new contract using the init code provided, with an address determined in part by the `salt`.
    /// Returns the new contract's address on success, or the error reason on failure.
    /// In both cases the EVM return data's length and the overall gas cost are returned too.
    /// Analogous to `vm.CREATE2`.
    fn create2(
        &mut self,
        code: Vec<u8>,
        endowment: Bytes32,
        salt: Bytes32,
        gas: u64,
    ) -> (Result<Bytes20>, u32, u64);

    /// Returns the EVM return data.
    /// Analogous to `vm.RETURNDATASIZE`.
    fn get_return_data(&mut self, offset: u32, size: u32) -> Vec<u8>;

    /// Emits an EVM log with the given number of topics and data, the first bytes of which should be the topic data.
    /// Returns an error message on failure.
    /// Analogous to `vm.LOG(n)` where n ∈ [0, 4].
    fn emit_log(&mut self, data: Vec<u8>, topics: u32) -> Result<()>;

    /// Emits a trace for the given opCode with no parameters.
    /// Use for the following hostios:
    /// env.args (CALLDATALOAD)
    /// env.evm_data.return_data_len (RETURNDATASIZE)
    /// account_balance (BALANCE)
    /// account_codehash (EXTCODEHASH)
    /// evm_gas_left (GAS)
    /// evm_ink_left (GAS)
    /// block_basefee (BASEFEE)
    /// chainid (CHAINID)
    /// block_coinbase (COINBASE)
    /// block_gas_limit (GASLIMIT)
    /// block_number (NUMBER)
    /// block_timestamp (TIMESTAMP)
    /// contract_address (ADDRESS)
    /// msg_sender (CALLER)
    /// msg_value (CALLVALUE)
    /// tx_gas_price (GASPRICE)
    /// tx_ink_price (GASPRICE)
    /// tx_origin (ORIGIN)
    fn report_hostio(&mut self, opcode: Opcode, gas: u64, cost: u64) -> Result<()>;

    /// Emits a trace for the given opCode with assorted parameters.
    /// Use for the following hostios:
    /// get_return_data (RETURNDATACOPY) - uses `offset` and `size`, ignores `data`
    /// native_keccak256 (SHA3) - uses `data`, ignores `offset` and `size`
    /// account_balance (BALANCE) - uses `data` (address), ignores `offset` and `size`
    /// account_balance (BALANCE) - uses `data` (address), ignores `offset` and `size`
    /// emit_log (LOG0-LOG4) - uses `data` and `size`, ignores `offset`
    fn report_hostio_advanced(
        &mut self,
        opcode: Opcode,
        data: Vec<u8>,
        offset: u32,
        size: u32,
        gas: u64,
        cost: u64,
    ) -> Result<()>;

    /// Gets the balance of the given account.
    /// Returns the balance and the access cost in gas.
    /// Analogous to `vm.BALANCE`.
    fn account_balance(&mut self, address: Bytes20) -> (Bytes32, u64);

    /// Gets the hash of the given address's code.
    /// Returns the hash and the access cost in gas.
    /// Analogous to `vm.CODEHASH`.
    fn account_codehash(&mut self, address: Bytes20) -> (Bytes32, u64);

    /// Determines the cost in gas of allocating additional wasm pages.
    /// Note: has the side effect of updating Geth's memory usage tracker.
    /// Not analogous to any EVM opcode.
    fn add_pages(&mut self, pages: u16) -> u64;
}
