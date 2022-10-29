// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

extern "C" {
    fn poly_wavm_gas_left() -> u64;
    fn poly_wavm_gas_status() -> u32;
    fn poly_wavm_set_gas(gas: u64, status: u32);
}

static mut GAS_PRICE: u64 = 1000;

pub(crate) unsafe fn buy_evm_gas(evm_gas: u64) {
    if poly_wavm_gas_status() != 0 {
        panic!("out of gas");
    }
    let mut gas_left = poly_wavm_gas_left();
    let gas_price = GAS_PRICE;

    let mut evm_gas_left = gas_left.saturating_mul(1000) / gas_price;
    if evm_gas > evm_gas_left {
        poly_wavm_set_gas(gas_left, 1);
        panic!("out of gas");
    }
    evm_gas_left -= evm_gas;
    gas_left = evm_gas_left.saturating_mul(1000) / gas_price;
    poly_wavm_set_gas(gas_left, 0);
}

pub unsafe fn set_gas_price(gas_price: u64) {
    GAS_PRICE = gas_price;
}
