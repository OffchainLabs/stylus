// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
// SPDX-License-Identifier: BUSL-1.1

pragma solidity >=0.4.21 <0.9.0;

/**
 * @title Methods for managing user programs
 * @notice Precompiled contract that exists in every Arbitrum chain at 0x00000000000000000000000000000000000000a0.
 */
interface ArbWASM {
    // @notice compile a wasm program
    // @param program the program to compile
    function compileProgram(address program) external;

    // @notice call a wasm program
    // @param id the program to call
    // @param data the calldata to pass to the wasm program
    // @return status whether the call succeeded (0 means success, nonzero failure)
    // @return result the output of the wasm program
    function callProgram(address program, bytes calldata data)
        external
        view
        returns (uint32 status, bytes memory result);
}
