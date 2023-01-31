// Copyright 2021-2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.0;

import "./ModuleMemory.sol";

contract Invariants {
    using ModuleMemoryLib for ModuleMemory;

    function proveLeaf(
        ModuleMemory memory mem,
        uint256 leafIdx,
        bytes calldata proof,
        uint256 startOffset
    )
        public
        pure
        returns (
            bytes32 contents,
            uint256 offset,
            MerkleProof memory merkle
        ) {
        return ModuleMemoryLib.proveLeaf(mem, leafIdx, proof, startOffset);
    }

    function pullLeafByte(bytes32 leaf, uint256 idx) public pure returns (uint8) {
        return ModuleMemoryLib.pullLeafByte(leaf, idx);
    }
}
