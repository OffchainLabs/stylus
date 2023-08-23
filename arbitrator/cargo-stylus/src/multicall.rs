use ethers::types::{H160, U256};

use crate::{constants::ARB_WASM_ADDRESS, deploy::activation_calldata};

pub fn check_deploy_compile_succeeds(compressed_wasm: &[u8], expected_address: &H160) -> Vec<u8> {
    let deployment_calldata = program_deployment_calldata(&compressed_wasm);

    let mut multicall_args =
        args_for_multicall(MulticallArg::Call, H160::zero(), None, deployment_calldata);

    let activate_calldata = activation_calldata(expected_address);
    let arbwasm_address = hex::decode(ARB_WASM_ADDRESS).unwrap();
    multicall_append(
        &mut multicall_args,
        MulticallArg::Call,
        H160::from_slice(&arbwasm_address),
        None,
        compile_calldata,
    );
    multicall_args
}

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
    args
}

fn multicall_append(
    calls: &mut Vec<u8>,
    opcode: MulticallArg,
    address: H160,
    value: Option<U256>,
    inner: Vec<u8>,
) {
    calls[0] += 1; // add another call
    let args = args_for_multicall(opcode, address, value, inner);
    calls.extend(args[1..].iter().cloned());
}
