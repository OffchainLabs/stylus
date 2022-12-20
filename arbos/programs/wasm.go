// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

//go:build js
// +build js

package programs

import (
	"errors"

	"github.com/offchainlabs/nitro/arbutil"
)

type u32 = uint32
type u64 = uint64

func polyglotCheck(wasm []byte) (status u64, output *byte, outlen, outcap u64)
func polyglotCall(wasm, calldata []byte, gas_price u64, gas *u64) (status u64, output *byte, outlen, outcap u64)
func polyglotCopy(dest, src *byte, length u64)
func polyglotFree(output *byte, outlen, outcap u64)

func polyCompile(wasm []byte) ([]byte, error) {
	status, outptr, outlen, outcap := polyglotCheck(wasm)
	defer polyglotFree(outptr, outlen, outcap)

	output := arbutil.PointerToSlice(outptr, int(outlen))
	if status != 0 {
		return nil, errors.New(string(output))
	}
	return output, nil
}

func polyCall(machine, calldata []byte, gas u64, gas_price u64) (u64, u32, []byte) {
	status, outptr, outlen, outcap := polyglotCall(machine, calldata, gas_price, &gas)
	output := make([]byte, outlen)
	polyglotCopy(arbutil.SliceToPointer(output), outptr, uint64(outlen))
	defer polyglotFree(outptr, outlen, outcap)

	return gas, u32(status), output
}
