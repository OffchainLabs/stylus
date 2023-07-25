// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
use bytesize::ByteSize;

/// EOF prefix used in Stylus compressed WASMs on-chain
pub const EOF_PREFIX: &'static str = "EF000000";
/// Maximum brotli compression level used for Stylus programs.
pub const BROTLI_COMPRESSION_LEVEL: u32 = 11;
/// Address of the Arbitrum WASM precompile on L2.
pub const ARB_WASM_ADDRESS: &'static str = "0000000000000000000000000000000000000071";
/// Address a multicall.rs Stylus program on L2.
pub const MULTICALL_ADDR: &'static str = "Eba70C09bA17508c75227cCACf376975490172c3";
/// Maximum allowed size of a program on Arbitrum (and Ethereum).
pub const MAX_PROGRAM_SIZE: ByteSize = ByteSize::kb(24);
