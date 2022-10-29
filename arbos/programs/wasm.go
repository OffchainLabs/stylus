// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

//go:build js
// +build js

package programs

import (
	"errors"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/log"
	"github.com/offchainlabs/nitro/arbutil"
)

func polyglotCheck(wasm []byte) (status uint64, output *byte, outlen, outcap uint64)
func polyglotCall(wasm, calldata []byte, gas_price uint64, output *byte, outlen, outcap, gas *uint64) (status uint64)
func polyglotFree(output *byte, outlen, outcap uint64)

func polyCompile(statedb vm.StateDB, program common.Address, wasm []byte) error {

	status, outptr, outlen, outcap := polyglotCheck(wasm)
	defer polyglotFree(outptr, outlen, outcap)

	output := arbutil.PointerToSlice(outptr, int(outlen))
	if status != 0 {
		return errors.New(string(output))
	}
	return nil
}

func polyCall(
	statedb vm.StateDB, program common.Address, calldata []byte, gas uint64, gas_price uint64,
) (uint64, uint64, []byte) {
	wasm, err := getWasm(statedb, program)
	if err != nil {
		log.Crit("failed to get wasm", "program", program, "err", err)
	}

	var outptr *byte
	var outlen, outcap *uint64

	status := polyglotCall(wasm, calldata, gas_price, outptr, outlen, outcap, &gas)
	defer polyglotFree(outptr, *outlen, *outcap)

	output := arbutil.PointerToSlice(outptr, int(*outlen))
	return gas, status, output
}
