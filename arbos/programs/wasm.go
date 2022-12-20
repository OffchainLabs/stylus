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

type u32 = uint32
type u64 = uint64

func polyglotCheck(wasm []byte) (status u64, output *byte, outlen, outcap u64)
func polyglotCall(wasm, calldata []byte, gas_price u64, gas *u64) (status u64, output *byte, outlen, outcap u64)
func polyglotCopy(dest, src *byte, length u64)
func polyglotFree(output *byte, outlen, outcap u64)

func polyCompile(statedb vm.StateDB, program common.Address, wasm []byte) error {
	status, outptr, outlen, outcap := polyglotCheck(wasm)
	defer polyglotFree(outptr, outlen, outcap)

	output := arbutil.PointerToSlice(outptr, int(outlen))
	if status != 0 {
		return errors.New(string(output))
	}
	return nil
}

func polyCall(statedb vm.StateDB, program common.Address, calldata []byte, gas u64, gas_price u64) (u64, u32, []byte) {
	wasm, err := getWasm(statedb, program)
	if err != nil {
		log.Crit("failed to get wasm", "program", program, "err", err)
	}

	status, outptr, outlen, outcap := polyglotCall(wasm, calldata, gas_price, &gas)
	output := make([]byte, outlen)
	polyglotCopy(arbutil.SliceToPointer(output), outptr, uint64(outlen))
	defer polyglotFree(outptr, outlen, outcap)

	return gas, u32(status), output
}
