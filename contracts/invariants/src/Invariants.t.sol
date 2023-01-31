// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.6;

import "ds-test/test.sol";

import "./Invariants.sol";
import "./ModuleMemory.sol";

contract InvariantsTest is DSTest {
    using ModuleMemoryLib for ModuleMemory;
    Invariants invariants;

    function setUp() public {
        invariants = new Invariants();
    }

    // Property tests.
    function test_pull_leaf_byte(bytes32 leaf, uint256 idx) public {
        invariants.pullLeafByte(leaf, idx);
    }

    function test_prove_leaf(
        ModuleMemory memory mem,
        uint256 leafIdx,
        bytes calldata proof,
        uint256 startOffset
    ) public {
        invariants.proveLeaf(mem, leafIdx, proof, startOffset);
    }

    // Symbolic execution that tries to find passing cases.
    function proveFail_prove_leaf(
        ModuleMemory memory mem,
        uint256 leafIdx,
        bytes calldata proof,
        uint256 startOffset
    ) public {
        invariants.proveLeaf(mem, leafIdx, proof, startOffset);
    }
}
