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

func callUserWasm(
	db vm.StateDB,
	program common.Address,
	calldata []byte,
	gas *uint64,
	params *goParams,
	evmContext *goEvmContext,
) ([]byte, error) {
	if db, ok := db.(*state.StateDB); ok {
		db.RecordProgram(program, params.version)
	}

	module := db.GetCompiledWasmCode(program, params.version)

	output := rustVec()
	status := userStatus(C.stylus_call(
		goSlice(module),
		goSlice(calldata),
		params.encode(),
		evmContext.encode(),
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

func (ec *goEvmContext) encode() C.GoEvmContext {
	var gasPrice u64
	if ec.gasPrice.IsUint64() {
		gasPrice = u64(ec.gasPrice.Uint64())
	}
	var baseFee u64
	if ec.baseFee.IsUint64() {
		baseFee = u64(ec.baseFee.Uint64())
	}

	time := common.BigToHash(ec.time)
	difficulty := common.BigToHash(ec.difficulty)
	return C.GoEvmContext{
		read_only:  boolToRustIntImpl(ec.readOnly),
		origin:     *addressToRustBytes20Impl(ec.origin),
		gas_price:  gasPrice,
		coinbase:   *addressToRustBytes20Impl(ec.coinbase),
		gas_limit:  u64(ec.gasLimit),
		time:       *hashToRustBytes32Impl(&time),
		difficulty: *hashToRustBytes32Impl(&difficulty),
		base_fee:   baseFee,
		random:     *hashToRustBytes32Impl(ec.random),
	}
}

func boolToRustIntImpl(b bool) u32 {
	if b {
		return 1
	}
	return 0
}

func addressToRustBytes20Impl(addr common.Address) *C.struct_Bytes20 {
	bytes := [20]C.uint8_t{}
	for index, current := range addr.Bytes() {
		bytes[index] = C.uint8_t(current)
	}

	return &C.struct_Bytes20{bytes}
}

func hashToRustBytes32Impl(hash *common.Hash) *C.struct_Bytes32 {
	bytes := [32]C.uint8_t{}
	for index, current := range hash.Bytes() {
		bytes[index] = C.uint8_t(current)
	}

	return &C.struct_Bytes32{bytes}
}
