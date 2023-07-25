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