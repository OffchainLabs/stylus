// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use ethers::types::{H160, U256};

#[derive(Clone)]
enum MulticallArg {
    Call,
    DelegateCall,
    StaticCall,
}

impl From<MulticallArg> for u8 {
    fn from(value: MulticallArg) -> Self {
        match value {
            MulticallArg::Call => 0x00,
            MulticallArg::DelegateCall => 0x01,
            MulticallArg::StaticCall => 0x02,
        }
    }
}

fn prepare_deploy_compile_multicall(compressed_wasm: &[u8], expected_address: &H160) -> Vec<u8> {
    // let code = contract_init_code(compressed_wasm);
    // let mut multicall_args = args_for_multicall(MulticallArg::Call, H160::zero(), None, code);
    // let arbwasm_address = hex::decode(constants::ARB_WASM_ADDRESS).unwrap();
    // let mut compile_calldata = vec![];
    // let compile_method_hash = hex::decode("2e50f32b").unwrap();
    // compile_calldata.extend(compile_method_hash);
    // compile_calldata.extend(hex::decode("000000000000000000000000").unwrap());
    // compile_calldata.extend(expected_address.as_bytes());
    // multicall_append(
    //     &mut multicall_args,
    //     MulticallArg::Call,
    //     H160::from_slice(&arbwasm_address),
    //     compile_calldata,
    // );
    vec![]
}

fn args_for_multicall(
    opcode: MulticallArg,
    address: H160,
    value: Option<U256>,
    calldata: Vec<u8>,
) -> Vec<u8> {
    let mut args = vec![0x01];
    let mut length: u32 = 21 + calldata.len() as u32;
    if matches!(opcode, MulticallArg::Call) {
        length += 32;
    }
    args.extend(length.to_be_bytes());
    args.push(opcode.clone().into());

    if matches!(opcode, MulticallArg::Call) {
        let mut val = [0u8; 32];
        value.unwrap_or(U256::zero()).to_big_endian(&mut val);
        args.extend(val);
    }
    args.extend(address.as_bytes());
    args.extend(calldata);
    println!("Got args as {}", hex::encode(&args));
    args
}

fn multicall_append(calls: &mut Vec<u8>, opcode: MulticallArg, address: H160, inner: Vec<u8>) {
    calls[0] += 1; // add another call
    let args = args_for_multicall(opcode, address, None, inner);
    calls.extend(args[1..].iter().cloned());
}
