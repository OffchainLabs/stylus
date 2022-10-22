// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
// SPDX-License-Identifier: BUSL-1.1

pragma solidity >=0.4.21 <0.9.0;

/**
 * @title Methods for managing user programs
 * @notice Precompiled contract that exists in every Arbitrum chain at 0x00000000000000000000000000000000000000a0.
 */
interface ArbWASM {
    // @notice upload a wasm program
    // @param wasm the program source
    // @return id the reference to program
    function addProgram(bytes calldata wasm) external returns (bytes32 wasm_hash);

    // @notice compile a wasm program
    // @param id the program to compile
    function compileProgram(bytes32 wasm_hash) external;

    // @notice call a wasm program
    // @param id the program to call
    // @param data the calldata to pass to the wasm program
    // @return status whether the call succeeded (0 means success, nonzero failure)
    // @return result the output of the wasm program
    function callProgram(bytes32 wasm_hash, bytes calldata data)
        external
        view
        returns (uint64 status, bytes memory result);

    event ProgramAdded(bytes32 indexed wasm_hash);
}
