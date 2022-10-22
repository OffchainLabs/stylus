// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

//go:build js
// +build js

package programs

func polyAddProgram(statedb vm.StateDB, hash common.Hash, wasm []byte) {
	// do nothing
}

func polyCompile(statedb vm.StateDB, wasm_hash common.Hash) error {
	// use the preimage oracle
}

func polyExecute(statedb vm.StateDB, wasm_hash common.Hash, calldata []byte, gas uint64) (uint64, uint64, []byte, error) {

	// get the program

	return 0, 0, nil, nil
}
