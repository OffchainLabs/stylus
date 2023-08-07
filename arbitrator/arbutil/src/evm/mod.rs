// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use crate::{Bytes20, Bytes32};

pub mod api;
pub mod js;
pub mod user;

// params.SstoreSentryGasEIP2200
pub const SSTORE_SENTRY_GAS: u64 = 2300;

// params.LogGas and params.LogDataGas
pub const LOG_TOPIC_GAS: u64 = 375;
pub const LOG_DATA_GAS: u64 = 8;

// params.CopyGas
pub const COPY_WORD_GAS: u64 = 3;

// params.Keccak256Gas
pub const KECCAK_256_GAS: u64 = 30;
pub const KECCAK_WORD_GAS: u64 = 6;

// vm.GasQuickStep (see gas.go)
pub const GAS_QUICK_STEP: u64 = 2;

// vm.GasQuickStep (see jump_table.go)
pub const ADDRESS_GAS: u64 = GAS_QUICK_STEP;

// vm.GasQuickStep (see eips.go)
pub const BASEFEE_GAS: u64 = GAS_QUICK_STEP;

// vm.GasQuickStep (see eips.go)
pub const CHAINID_GAS: u64 = GAS_QUICK_STEP;

// vm.GasQuickStep (see jump_table.go)
pub const COINBASE_GAS: u64 = GAS_QUICK_STEP;

// vm.GasQuickStep (see jump_table.go)
pub const GASLIMIT_GAS: u64 = GAS_QUICK_STEP;

// vm.GasQuickStep (see jump_table.go)
pub const NUMBER_GAS: u64 = GAS_QUICK_STEP;

// vm.GasQuickStep (see jump_table.go)
pub const TIMESTAMP_GAS: u64 = GAS_QUICK_STEP;

// vm.GasQuickStep (see jump_table.go)
pub const GASLEFT_GAS: u64 = GAS_QUICK_STEP;

// vm.GasQuickStep (see jump_table.go)
pub const CALLER_GAS: u64 = GAS_QUICK_STEP;

// vm.GasQuickStep (see jump_table.go)
pub const CALLVALUE_GAS: u64 = GAS_QUICK_STEP;

// vm.GasQuickStep (see jump_table.go)
pub const GASPRICE_GAS: u64 = GAS_QUICK_STEP;

// vm.GasQuickStep (see jump_table.go)
pub const RETURNDATASIZE_GAS: u64 = GAS_QUICK_STEP;

// vm.GasQuickStep (see jump_table.go)
pub const ORIGIN_GAS: u64 = GAS_QUICK_STEP;

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum Opcode {
    KECCAK256 = 0x20,
    ADDRESS = 0x30,
    BALANCE = 0x31,
    ORIGIN = 0x32,
    CALLER = 0x33,
    CALLVALUE = 0x34,
    CALLDATALOAD = 0x35,
    CALLDATASIZE = 0x36,
    CALLDATACOPY = 0x37,
    GASPRICE = 0x3A,
    RETURNDATASIZE = 0x3D,
    RETURNDATACOPY = 0x3E,
    EXTCODEHASH = 0x3f,
    COINBASE = 0x41,
    TIMESTAMP = 0x42,
    NUMBER = 0x43,
    GASLIMIT = 0x45,
    CHAINID = 0x46,
    BASEFEE = 0x48,
    GAS = 0x5A,
    LOG0 = 0xA0,
    RETURN = 0xF3,
    REVERT = 0xFD,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct EvmData {
    pub block_basefee: Bytes32,
    pub chainid: Bytes32,
    pub block_coinbase: Bytes20,
    pub block_gas_limit: u64,
    pub block_number: Bytes32,
    pub block_timestamp: u64,
    pub contract_address: Bytes20,
    pub msg_sender: Bytes20,
    pub msg_value: Bytes32,
    pub tx_gas_price: Bytes32,
    pub tx_origin: Bytes20,
    pub tracing_enabled: u8,
    pub return_data_len: u32,
}

/// Returns the minimum number of EVM words needed to store `bytes` bytes.
pub fn evm_words(bytes: u64) -> u64 {
    match bytes % 32 {
        0 => bytes / 32,
        _ => bytes / 32 + 1,
    }
}
