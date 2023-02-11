// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

//go:build !js
// +build !js

package programs

/*
#cgo CFLAGS: -g -Wall -I../../target/include/
#cgo LDFLAGS: ${SRCDIR}/../../target/lib/libstylus.a -ldl -lm
#include "arbitrator-stylus.h"
*/
import "C"
import (
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/state"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/offchainlabs/nitro/arbutil"
)

type u8 = C.uint8_t
type u32 = C.uint32_t
type u64 = C.uint64_t
type usize = C.size_t

func compileUserWasm(db vm.StateDB, program common.Address, wasm []byte, version uint32) error {
	output := rustVec()
	status := userStatus(C.stylus_compile(
		goSlice(wasm),
		u32(version),
		output,
	))
	result, err := status.output(output.read())
	if err == nil {
		db.SetCompiledWasmCode(program, result, version)
	}
	return err
}

func callUserWasm(db vm.StateDB, program common.Address, calldata []byte, gas *uint64, params *goParams) ([]byte, error) {
	if db, ok := db.(*state.StateDB); ok {
		db.RecordProgram(program, params.version)
	}

	module := db.GetCompiledWasmCode(program, params.version)

	output := rustVec()
	status := userStatus(C.stylus_call(
		goSlice(module),
		goSlice(calldata),
		params.encode(),
		output,
		(*u64)(gas),
	))
	return status.output(output.read())
}

func rustVec() C.RustVec {
	var ptr *u8
	var len usize
	var cap usize
	return C.RustVec{
		ptr: (**u8)(&ptr),
		len: (*usize)(&len),
		cap: (*usize)(&cap),
	}
}

func (vec C.RustVec) read() []byte {
	slice := arbutil.PointerToSlice((*byte)(*vec.ptr), int(*vec.len))
	C.stylus_free(vec)
	return slice
}

func goSlice(slice []byte) C.GoSlice {
	return C.GoSlice{
		ptr: (*u8)(arbutil.SliceToPointer(slice)),
		len: usize(len(slice)),
	}
}

func (params *goParams) encode() C.GoParams {
	return C.GoParams{
		version:        u32(params.version),
		max_depth:      u32(params.maxDepth),
		wasm_gas_price: u64(params.wasmGasPrice),
		hostio_cost:    u64(params.hostioCost),
	}
}
