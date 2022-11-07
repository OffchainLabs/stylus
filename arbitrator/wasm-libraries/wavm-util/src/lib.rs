// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

extern "C" {
    pub fn wavm_caller_load8(ptr: usize) -> u8;
    pub fn wavm_caller_load32(ptr: usize) -> u32;
    pub fn wavm_caller_store8(ptr: usize, val: u8);
    pub fn wavm_caller_store32(ptr: usize, val: u32);
}

pub unsafe fn wavm_caller_load64(ptr: usize) -> u64 {
    let lower = wavm_caller_load32(ptr);
    let upper = wavm_caller_load32(ptr + 4);
    lower as u64 | ((upper as u64) << 32)
}

pub unsafe fn wavm_caller_store64(ptr: usize, val: u64) {
    wavm_caller_store32(ptr, val as u32);
    wavm_caller_store32(ptr + 4, (val >> 32) as u32);
}

pub unsafe fn write_slice(mut src: &[u8], mut ptr: usize) {
    while src.len() >= 4 {
        let mut arr = [0u8; 4];
        arr.copy_from_slice(&src[..4]);
        wavm_caller_store32(ptr, u32::from_le_bytes(arr));
        ptr += 4;
        src = &src[4..];
    }
    for &byte in src {
        wavm_caller_store8(ptr, byte);
        ptr += 1;
    }
}

pub unsafe fn read_slice(mut ptr: usize, mut len: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(len as usize);
    if len == 0 {
        return data;
    }
    while len >= 4 {
        data.extend(wavm_caller_load32(ptr).to_le_bytes());
        ptr += 4;
        len -= 4;
    }
    for _ in 0..len {
        data.push(wavm_caller_load8(ptr));
        ptr += 1;
    }
    data
}
