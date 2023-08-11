;; Copyright 2022-2023, Offchain Labs, Inc.
;; For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

(module
    (import "user_host" "arbitrator_forward__read_args"             (func $read_args             (param i32)))
    (import "user_host" "arbitrator_forward__write_result"          (func $write_result          (param i32 i32)))
    (import "user_host" "arbitrator_forward__storage_load_bytes32"  (func $storage_load_bytes32  (param i32 i32)))
    (import "user_host" "arbitrator_forward__storage_store_bytes32" (func $storage_store_bytes32 (param i32 i32)))
    (import "user_host" "arbitrator_forward__call_contract"
        (func $call_contract (param i32 i32 i32 i32 i64 i32) (result i32)))
    (import "user_host" "arbitrator_forward__delegate_call_contract"
        (func $delegate_call (param i32 i32 i32 i64 i32) (result i32)))
    (import "user_host" "arbitrator_forward__static_call_contract"
        (func $static_call   (param i32 i32 i32 i64 i32) (result i32)))
    (import "user_host" "arbitrator_forward__create1"          (func $create1 (param i32 i32 i32 i32 i32)))
    (import "user_host" "arbitrator_forward__create2"          (func $create2 (param i32 i32 i32 i32 i32 i32)))
    (import "user_host" "arbitrator_forward__read_return_data" (func $read_return_data (param i32 i32 i32) (result i32)))
    (import "user_host" "arbitrator_forward__return_data_size" (func $return_data_size (result i32)))
    (import "user_host" "arbitrator_forward__emit_log"         (func $emit_log         (param i32 i32 i32)))
    (import "user_host" "arbitrator_forward__report_hostio"    (func $report_hostio    (param i32 i64 i64)))
    (import "user_host" "arbitrator_forward__report_hostio_advanced"
        (func $report_hostio_advanced (param i32 i32 i32 i32 i32 i64 i64)))
    (import "user_host" "arbitrator_forward__account_balance"  (func $account_balance  (param i32 i32)))
    (import "user_host" "arbitrator_forward__account_codehash" (func $account_codehash (param i32 i32)))
    (import "user_host" "arbitrator_forward__evm_gas_left"     (func $evm_gas_left     (result i64)))
    (import "user_host" "arbitrator_forward__evm_ink_left"     (func $evm_ink_left     (result i64)))
    (import "user_host" "arbitrator_forward__block_basefee"    (func $block_basefee    (param i32)))
    (import "user_host" "arbitrator_forward__chainid"          (func $chainid          (result i64)))
    (import "user_host" "arbitrator_forward__block_coinbase"   (func $block_coinbase   (param i32)))
    (import "user_host" "arbitrator_forward__block_gas_limit"  (func $block_gas_limit  (result i64)))
    (import "user_host" "arbitrator_forward__block_number"     (func $block_number     (result i64)))
    (import "user_host" "arbitrator_forward__block_timestamp"  (func $block_timestamp  (result i64)))
    (import "user_host" "arbitrator_forward__contract_address" (func $contract_address (param i32)))
    (import "user_host" "arbitrator_forward__msg_reentrant"    (func $msg_reentrant    (result i32)))
    (import "user_host" "arbitrator_forward__msg_sender"       (func $msg_sender       (param i32)))
    (import "user_host" "arbitrator_forward__msg_value"        (func $msg_value        (param i32)))
    (import "user_host" "arbitrator_forward__native_keccak256" (func $native_keccak256 (param i32 i32 i32)))
    (import "user_host" "arbitrator_forward__tx_gas_price"     (func $tx_gas_price     (param i32)))
    (import "user_host" "arbitrator_forward__tx_ink_price"     (func $tx_ink_price     (result i32)))
    (import "user_host" "arbitrator_forward__tx_origin"        (func $tx_origin        (param i32)))
    (import "user_host" "arbitrator_forward__memory_grow"      (func $memory_grow      (param i32)))
    (export "vm_hooks__read_args"              (func $read_args))
    (export "vm_hooks__write_result"           (func $write_result))
    (export "vm_hooks__storage_load_bytes32"   (func $storage_load_bytes32))
    (export "vm_hooks__storage_store_bytes32"  (func $storage_store_bytes32))
    (export "vm_hooks__call_contract"          (func $call_contract))
    (export "vm_hooks__delegate_call_contract" (func $delegate_call))
    (export "vm_hooks__static_call_contract"   (func $static_call))
    (export "vm_hooks__create1"                (func $create1))
    (export "vm_hooks__create2"                (func $create2))
    (export "vm_hooks__read_return_data"       (func $read_return_data))
    (export "vm_hooks__return_data_size"       (func $return_data_size))
    (export "vm_hooks__emit_log"               (func $emit_log))
    (export "vm_hooks__report_hostio"          (func $report_hostio))
    (export "vm_hooks__report_hostio_advanced" (func $report_hostio_advanced))
    (export "vm_hooks__account_balance"        (func $account_balance))
    (export "vm_hooks__account_codehash"       (func $account_codehash))
    (export "vm_hooks__evm_gas_left"           (func $evm_gas_left))
    (export "vm_hooks__evm_ink_left"           (func $evm_ink_left))
    (export "vm_hooks__block_basefee"          (func $block_basefee))
    (export "vm_hooks__chainid"                (func $chainid))
    (export "vm_hooks__block_coinbase"         (func $block_coinbase))
    (export "vm_hooks__block_gas_limit"        (func $block_gas_limit))
    (export "vm_hooks__block_number"           (func $block_number))
    (export "vm_hooks__block_timestamp"        (func $block_timestamp))
    (export "vm_hooks__contract_address"       (func $contract_address))
    (export "vm_hooks__msg_reentrant"          (func $msg_reentrant))
    (export "vm_hooks__msg_sender"             (func $msg_sender))
    (export "vm_hooks__msg_value"              (func $msg_value))
    (export "vm_hooks__native_keccak256"       (func $native_keccak256))
    (export "vm_hooks__tx_gas_price"           (func $tx_gas_price))
    (export "vm_hooks__tx_ink_price"           (func $tx_ink_price))
    (export "vm_hooks__tx_origin"              (func $tx_origin))
    (export "vm_hooks__memory_grow"            (func $memory_grow))
)
