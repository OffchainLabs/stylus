// Copyright 2022-2023, Offchain Labs, Inc.
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

type addr = common.Address
type hash = common.Hash

// rust types
type u8 = uint8
type u32 = uint32
type u64 = uint64
type usize = uintptr

// opaque types
type Bytes20 byte
type Bytes32 byte
type rustVec byte
type rustConfig byte
type rustMachine byte
type rustEvmContext byte

func compileUserWasmRustImpl(wasm []byte, version u32) (machine *rustMachine, err *rustVec)
func callUserWasmRustImpl(machine *rustMachine, calldata []byte, params *rustConfig, evmContext *rustEvmContext, gas *u64, root *hash) (status userStatus, out *rustVec)
func readRustVecLenImpl(vec *rustVec) (len u32)
func rustVecIntoSliceImpl(vec *rustVec, ptr *byte)
func rustConfigImpl(version, maxDepth u32, wasmGasPrice, hostioCost u64) *rustConfig
func boolToRustIntImpl(b bool) u32
func addressToRustBytes20Impl(addr common.Address) *Bytes20
func hashToRustBytes32Impl(hash *common.Hash) *Bytes32
func rustEvmContextImpl(readOnly u32, origin *Bytes20, gasPrice u64, coinbase *Bytes20, gasLimit u64, time *Bytes32, difficulty *Bytes32, baseFee u64, random *Bytes32) *rustEvmContext

func compileUserWasm(db vm.StateDB, program addr, wasm []byte, version uint32) error {
	_, err := compileMachine(db, program, wasm, version)
	return err
}

func callUserWasm(db vm.StateDB, program addr, calldata []byte, gas *uint64, params *goParams, evmContext *goEvmContext) ([]byte, error) {
	wasm, err := getWasm(db, program)
	if err != nil {
		log.Crit("failed to get wasm", "program", program, "err", err)
	}
	machine, err := compileMachine(db, program, wasm, params.version)
	if err != nil {
		log.Crit("failed to create machine", "program", program, "err", err)
	}
	root := db.NoncanonicalProgramHash(program, params.version)
	return machine.call(calldata, params, evmContext, gas, &root)
}

func compileMachine(db vm.StateDB, program addr, wasm []byte, version uint32) (*rustMachine, error) {
	machine, err := compileUserWasmRustImpl(wasm, version)
	if err != nil {
		return nil, errors.New(string(err.intoSlice()))
	}
	return machine, nil
}

func (m *rustMachine) call(calldata []byte, params *goParams, evmContext *goEvmContext, gas *u64, root *hash) ([]byte, error) {
	status, output := callUserWasmRustImpl(m, calldata, params.encode(), evmContext.encode(), gas, root)
	result := output.intoSlice()
	return status.output(result)
}

func (vec *rustVec) intoSlice() []byte {
	len := readRustVecLenImpl(vec)
	slice := make([]byte, len)
	rustVecIntoSliceImpl(vec, arbutil.SliceToPointer(slice))
	return slice
}

func (p *goParams) encode() *rustConfig {
	return rustConfigImpl(p.version, p.maxDepth, p.wasmGasPrice, p.hostioCost)
}

func (ec *goEvmContext) encode() *rustEvmContext {
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
	return rustEvmContextImpl(
		boolToRustIntImpl(ec.readOnly),
		addressToRustBytes20Impl(ec.origin),
		gasPrice,
		addressToRustBytes20Impl(ec.coinbase),
		u64(ec.gasLimit),
		hashToRustBytes32Impl(&time),
		hashToRustBytes32Impl(&difficulty),
		baseFee,
		hashToRustBytes32Impl(ec.random),
	)
}
