// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#![allow(clippy::too_many_arguments)]

use crate::env::{Escape, MaybeEscape, WasmEnv, WasmEnvMut};
use arbutil::{
    crypto,
    evm::{self, api::EvmApi, user::UserOutcomeKind, Opcode},
    pricing::{EVM_API_INK, HOSTIO_INK, PTR_INK},
    Bytes20, Bytes32,
};
use prover::{programs::prelude::*, value::Value};

pub(crate) fn read_args<E: EvmApi>(mut env: WasmEnvMut<E>, ptr: u32) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, 0)?;
    env.pay_for_write(env.args.len() as u64)?;
    env.report_hostio(Opcode::CALLDATALOAD)?;
    env.write_slice(ptr, &env.args)?;
    Ok(())
}

pub(crate) fn write_result<E: EvmApi>(mut env: WasmEnvMut<E>, ptr: u32, len: u32) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, 0)?;
    env.pay_for_read(len.into())?;
    // No trace report for write_result
    env.outs = env.read_slice(ptr, len)?;
    Ok(())
}

pub(crate) fn storage_load_bytes32<E: EvmApi>(
    mut env: WasmEnvMut<E>,
    key: u32,
    dest: u32,
) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, 2 * PTR_INK + EVM_API_INK)?;
    let key = env.read_bytes32(key)?;
    let (value, gas_cost) = env.evm_api.get_bytes32(key);
    // Trace already reported in get_bytes32, cost does not include gas consumed by WasmEnv::start
    env.buy_gas(gas_cost)?;
    env.write_bytes32(dest, value)?;
    Ok(())
}

pub(crate) fn storage_store_bytes32<E: EvmApi>(
    mut env: WasmEnvMut<E>,
    key: u32,
    value: u32,
) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, 2 * PTR_INK + EVM_API_INK)?;
    env.require_gas(evm::SSTORE_SENTRY_GAS)?; // see operations_acl_arbitrum.go

    let key = env.read_bytes32(key)?;
    let value = env.read_bytes32(value)?;
    let gas_cost = env.evm_api.set_bytes32(key, value)?;
    // Trace already reported in set_bytes32, cost does not include gas consumed by WasmEnv::start
    env.buy_gas(gas_cost)?;
    Ok(())
}

pub(crate) fn call_contract<E: EvmApi>(
    env: WasmEnvMut<E>,
    contract: u32,
    data: u32,
    data_len: u32,
    value: u32,
    gas: u64,
    ret_len: u32,
) -> Result<u8, Escape> {
    let value = Some(value);
    let call = |api: &mut E, contract, data, gas, value: Option<_>| {
        api.contract_call(contract, data, gas, value.unwrap())
    };
    do_call(env, contract, data, data_len, value, gas, ret_len, call)
}

pub(crate) fn delegate_call_contract<E: EvmApi>(
    env: WasmEnvMut<E>,
    contract: u32,
    data: u32,
    data_len: u32,
    gas: u64,
    ret_len: u32,
) -> Result<u8, Escape> {
    let call = |api: &mut E, contract, data, gas, _| api.delegate_call(contract, data, gas);
    do_call(env, contract, data, data_len, None, gas, ret_len, call)
}

pub(crate) fn static_call_contract<E: EvmApi>(
    env: WasmEnvMut<E>,
    contract: u32,
    data: u32,
    data_len: u32,
    gas: u64,
    ret_len: u32,
) -> Result<u8, Escape> {
    let call = |api: &mut E, contract, data, gas, _| api.static_call(contract, data, gas);
    do_call(env, contract, data, data_len, None, gas, ret_len, call)
}

pub(crate) fn do_call<F, E>(
    mut env: WasmEnvMut<E>,
    contract: u32,
    calldata: u32,
    calldata_len: u32,
    value: Option<u32>,
    mut gas: u64,
    return_data_len: u32,
    call: F,
) -> Result<u8, Escape>
where
    E: EvmApi,
    F: FnOnce(&mut E, Bytes20, Vec<u8>, u64, Option<Bytes32>) -> (u32, u64, UserOutcomeKind),
{
    let mut env = WasmEnv::start(&mut env, 3 * PTR_INK + EVM_API_INK)?;
    env.pay_for_read(calldata_len.into())?;
    gas = gas.min(env.gas_left()?); // provide no more than what the user has

    let contract = env.read_bytes20(contract)?;
    let input = env.read_slice(calldata, calldata_len)?;
    let value = value.map(|x| env.read_bytes32(x)).transpose()?;
    let api = &mut env.evm_api;

    let (outs_len, gas_cost, status) = call(api, contract, input, gas, value);
    env.buy_gas(gas_cost)?;
    env.evm_data.return_data_len = outs_len;
    env.write_u32(return_data_len, outs_len)?;
    // Trace already reported in call, cost does not include gas consumed by WasmEnv::start
    Ok(status as u8)
}

pub(crate) fn create1<E: EvmApi>(
    mut env: WasmEnvMut<E>,
    code: u32,
    code_len: u32,
    endowment: u32,
    contract: u32,
    revert_data_len: u32,
) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, 3 * PTR_INK + EVM_API_INK)?;
    env.pay_for_read(code_len.into())?;

    let code = env.read_slice(code, code_len)?;
    let endowment = env.read_bytes32(endowment)?;
    let gas = env.gas_left()?;

    let (result, ret_len, gas_cost) = env.evm_api.create1(code, endowment, gas);
    env.buy_gas(gas_cost)?;
    env.evm_data.return_data_len = ret_len;
    env.write_u32(revert_data_len, ret_len)?;
    // Trace already emitted in create1, cost does not include gas consumed by WasmEnv::start
    env.write_bytes20(contract, result?)?;
    Ok(())
}

pub(crate) fn create2<E: EvmApi>(
    mut env: WasmEnvMut<E>,
    code: u32,
    code_len: u32,
    endowment: u32,
    salt: u32,
    contract: u32,
    revert_data_len: u32,
) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, 4 * PTR_INK + EVM_API_INK)?;
    env.pay_for_read(code_len.into())?;

    let code = env.read_slice(code, code_len)?;
    let endowment = env.read_bytes32(endowment)?;
    let salt = env.read_bytes32(salt)?;
    let gas = env.gas_left()?;

    let (result, ret_len, gas_cost) = env.evm_api.create2(code, endowment, salt, gas);
    env.buy_gas(gas_cost)?;
    env.evm_data.return_data_len = ret_len;
    env.write_u32(revert_data_len, ret_len)?;
    // Trace already emitted in create2, cost does not include gas consumed by WasmEnv::start
    env.write_bytes20(contract, result?)?;
    Ok(())
}

pub(crate) fn read_return_data<E: EvmApi>(
    mut env: WasmEnvMut<E>,
    dest: u32,
    offset: u32,
    size: u32,
) -> Result<u32, Escape> {
    let mut env = WasmEnv::start(&mut env, EVM_API_INK)?;
    env.pay_for_write(size.into())?;

    let data = env.evm_api.get_return_data(offset, size);
    assert!(data.len() <= size as usize);
    env.report_hostio_advanced(Opcode::RETURNDATACOPY, &data, offset, size)?;
    env.write_slice(dest, &data)?;
    Ok(data.len() as u32)
}

pub(crate) fn return_data_size<E: EvmApi>(mut env: WasmEnvMut<E>) -> Result<u32, Escape> {
    let mut env = WasmEnv::start(&mut env, 0)?;
    env.report_hostio(Opcode::RETURNDATASIZE)?;
    let len = env.evm_data.return_data_len;
    Ok(len)
}

pub(crate) fn emit_log<E: EvmApi>(
    mut env: WasmEnvMut<E>,
    data: u32,
    len: u32,
    topics: u32,
) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, EVM_API_INK)?;
    if topics > 4 || len < topics * 32 {
        return Escape::logical("bad topic data");
    }
    env.pay_for_read(len.into())?;
    env.pay_for_evm_log(topics, len - topics * 32)?;

    let data = env.read_slice(data, len)?;
    // LOG0 is just placeholder, trace will be created with appropriate LOG0-LOG4
    env.report_hostio_advanced(Opcode::LOG0, &data, 0, topics)?;
    env.evm_api.emit_log(data, topics)?;
    Ok(())
}

pub(crate) fn account_balance<E: EvmApi>(
    mut env: WasmEnvMut<E>,
    address: u32,
    ptr: u32,
) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, 2 * PTR_INK + EVM_API_INK)?;
    let address = env.read_bytes20(address)?;
    let (balance, gas_cost) = env.evm_api.account_balance(address);
    env.buy_gas(gas_cost)?;
    env.report_hostio_advanced(Opcode::BALANCE, &address.as_slice(), 0, 0)?;
    env.write_bytes32(ptr, balance)?;
    Ok(())
}

pub(crate) fn account_codehash<E: EvmApi>(
    mut env: WasmEnvMut<E>,
    address: u32,
    ptr: u32,
) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, 2 * PTR_INK + EVM_API_INK)?;
    let address = env.read_bytes20(address)?;
    let (hash, gas_cost) = env.evm_api.account_codehash(address);
    env.buy_gas(gas_cost)?;
    env.report_hostio_advanced(Opcode::EXTCODEHASH, &address.as_slice(), 0, 0)?;
    env.write_bytes32(ptr, hash)?;
    Ok(())
}

pub(crate) fn evm_gas_left<E: EvmApi>(mut env: WasmEnvMut<E>) -> Result<u64, Escape> {
    let mut env = WasmEnv::start(&mut env, 0)?;
    env.report_hostio(Opcode::GAS)?;
    Ok(env.gas_left()?)
}

pub(crate) fn evm_ink_left<E: EvmApi>(mut env: WasmEnvMut<E>) -> Result<u64, Escape> {
    let mut env = WasmEnv::start(&mut env, 0)?;
    env.report_hostio(Opcode::GAS)?;
    Ok(env.ink_ready()?)
}

pub(crate) fn block_basefee<E: EvmApi>(mut env: WasmEnvMut<E>, ptr: u32) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, PTR_INK)?;
    env.report_hostio(Opcode::BASEFEE)?;
    env.write_bytes32(ptr, env.evm_data.block_basefee)?;
    Ok(())
}

pub(crate) fn chainid<E: EvmApi>(mut env: WasmEnvMut<E>) -> Result<u64, Escape> {
    let mut env = WasmEnv::start(&mut env, 0)?;
    env.report_hostio(Opcode::CHAINID)?;
    Ok(env.evm_data.chainid)
}

pub(crate) fn block_coinbase<E: EvmApi>(mut env: WasmEnvMut<E>, ptr: u32) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, PTR_INK)?;
    env.report_hostio(Opcode::COINBASE)?;
    env.write_bytes20(ptr, env.evm_data.block_coinbase)?;
    Ok(())
}

pub(crate) fn block_gas_limit<E: EvmApi>(mut env: WasmEnvMut<E>) -> Result<u64, Escape> {
    let mut env = WasmEnv::start(&mut env, 0)?;
    env.report_hostio(Opcode::GASLIMIT)?;
    Ok(env.evm_data.block_gas_limit)
}

pub(crate) fn block_number<E: EvmApi>(mut env: WasmEnvMut<E>) -> Result<u64, Escape> {
    let mut env = WasmEnv::start(&mut env, 0)?;
    env.report_hostio(Opcode::NUMBER)?;
    Ok(env.evm_data.block_number)
}

pub(crate) fn block_timestamp<E: EvmApi>(mut env: WasmEnvMut<E>) -> Result<u64, Escape> {
    let mut env = WasmEnv::start(&mut env, 0)?;
    env.report_hostio(Opcode::TIMESTAMP)?;
    Ok(env.evm_data.block_timestamp)
}

pub(crate) fn contract_address<E: EvmApi>(mut env: WasmEnvMut<E>, ptr: u32) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, PTR_INK)?;
    env.report_hostio(Opcode::ADDRESS)?;
    env.write_bytes20(ptr, env.evm_data.contract_address)?;
    Ok(())
}

pub(crate) fn msg_reentrant<E: EvmApi>(mut env: WasmEnvMut<E>) -> Result<u32, Escape> {
    let mut env = WasmEnv::start(&mut env, 0)?;
    Ok(env.evm_data.reentrant as u32)
}

pub(crate) fn msg_sender<E: EvmApi>(mut env: WasmEnvMut<E>, ptr: u32) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, PTR_INK)?;
    env.report_hostio(Opcode::CALLER)?;
    env.write_bytes20(ptr, env.evm_data.msg_sender)?;
    Ok(())
}

pub(crate) fn msg_value<E: EvmApi>(mut env: WasmEnvMut<E>, ptr: u32) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, PTR_INK)?;
    env.report_hostio(Opcode::CALLVALUE)?;
    env.write_bytes32(ptr, env.evm_data.msg_value)?;
    Ok(())
}

pub(crate) fn native_keccak256<E: EvmApi>(
    mut env: WasmEnvMut<E>,
    input: u32,
    len: u32,
    output: u32,
) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, PTR_INK)?;
    env.pay_for_keccak(len.into())?;

    let preimage = env.read_slice(input, len)?;
    env.report_hostio_advanced(Opcode::KECCAK256, &preimage, 0, 0)?;
    let digest = crypto::keccak(preimage);
    env.write_bytes32(output, digest.into())?;
    Ok(())
}

pub(crate) fn tx_gas_price<E: EvmApi>(mut env: WasmEnvMut<E>, ptr: u32) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, PTR_INK)?;
    env.report_hostio(Opcode::GASPRICE)?;
    env.write_bytes32(ptr, env.evm_data.tx_gas_price)?;
    Ok(())
}

pub(crate) fn tx_ink_price<E: EvmApi>(mut env: WasmEnvMut<E>) -> Result<u32, Escape> {
    let mut env = WasmEnv::start(&mut env, 0)?;
    env.report_hostio(Opcode::GASPRICE)?;
    Ok(env.pricing().ink_price)
}

pub(crate) fn tx_origin<E: EvmApi>(mut env: WasmEnvMut<E>, ptr: u32) -> MaybeEscape {
    let mut env = WasmEnv::start(&mut env, PTR_INK)?;
    env.report_hostio(Opcode::ORIGIN)?;
    env.write_bytes20(ptr, env.evm_data.tx_origin)?;
    Ok(())
}

pub(crate) fn memory_grow<E: EvmApi>(mut env: WasmEnvMut<E>, pages: u16) -> MaybeEscape {
    let mut env = WasmEnv::start_free(&mut env);
    if pages == 0 {
        env.buy_ink(HOSTIO_INK)?;
        return Ok(());
    }
    let gas_cost = env.evm_api.add_pages(pages);
    env.buy_gas(gas_cost)?;
    Ok(())
}

pub(crate) fn console_log_text<E: EvmApi>(
    mut env: WasmEnvMut<E>,
    ptr: u32,
    len: u32,
) -> MaybeEscape {
    let env = WasmEnv::start_free(&mut env);
    let text = env.read_slice(ptr, len)?;
    env.say(String::from_utf8_lossy(&text));
    Ok(())
}

pub(crate) fn console_log<E: EvmApi, T: Into<Value>>(
    mut env: WasmEnvMut<E>,
    value: T,
) -> MaybeEscape {
    let env = WasmEnv::start_free(&mut env);
    env.say(value.into());
    Ok(())
}

pub(crate) fn console_tee<E: EvmApi, T: Into<Value> + Copy>(
    mut env: WasmEnvMut<E>,
    value: T,
) -> Result<T, Escape> {
    let env = WasmEnv::start_free(&mut env);
    env.say(value.into());
    Ok(value)
}

pub(crate) fn null_host<E: EvmApi>(_: WasmEnvMut<E>) {}
