// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package programs

import (
	"errors"
	"fmt"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/state"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/log"
	"github.com/offchainlabs/nitro/arbcompress"
	"github.com/offchainlabs/nitro/arbos/storage"
)

const (
	MaxWASMSize     = 64 * 1024
	polyglotVersion = 1
)

type Programs struct {
	backingStorage  *storage.Storage
	machineVersions *storage.Storage
	wasmGasPrice    *storage.StorageBackedUint64
}

func Initialize(sto *storage.Storage) {
	wasmGasPrice := sto.OpenStorageBackedUint64(0)
	_ = wasmGasPrice.Set(1000)
}

func Open(sto *storage.Storage) *Programs {
	machineInfo := sto.OpenSubStorage([]byte{})
	wasmGasPrice := sto.OpenStorageBackedUint64(0)
	return &Programs{sto, machineInfo, &wasmGasPrice}
}

func Call(programs *Programs, inputGas uint64, evm *vm.EVM, address common.Address, calldata []byte) (uint32, []byte, error) {
	// TODO: require some intrinsic amount of gas
	// give all gas to the program
	getGas := func() uint64 { return inputGas }
	gasLeft, status, output, err := programs.CallProgram(evm.StateDB, address, calldata, getGas)
	if err != nil {
		return 0, nil, err
	}

	if gasLeft > gasLeft {
		log.Error("program gas didn't decrease", "gas", gasLeft, "gasLeft", gasLeft)
		return 0, nil, errors.New("internal metering error")
	}
	return status, output, nil
}

func (p Programs) CompileProgram(statedb vm.StateDB, addr common.Address) error {
	wasm, err := getWasm(statedb, addr)
	if err != nil {
		return err
	}
	machineOutput, err := polyCompile(wasm)
	if err != nil {
		return err
	}
	// Add the machine output to ArbDB.
	statedb.AddPolyMachine(polyglotVersion, addr, machineOutput)
	return p.machineVersions.SetUint64(addr.Hash(), 1)
}

func (p Programs) CallProgram(
	statedb vm.StateDB,
	program common.Address,
	calldata []byte,
	gas func() uint64,
) (uint64, uint32, []byte, error) {
	version, err := p.machineVersions.GetUint64(program.Hash())
	if err != nil {
		return 0, 0, nil, err
	}
	if version == 0 {
		return 0, 0, nil, errors.New("wasm not compiled")
	}
	gasPrice, err := p.wasmGasPrice.Get()
	if err != nil {
		return 0, 0, nil, err
	}
	if db, ok := statedb.(*state.StateDB); ok {
		db.RecordProgram(program)
	}
	machine, err := statedb.GetPolyMachine(polyglotVersion, program)
	if err != nil {
		return 0, 0, nil, err
	}
	gasLeft, status, output := polyCall(machine, calldata, gas(), gasPrice)
	return gasLeft, status, output, nil
}

func getWasm(statedb vm.StateDB, addr common.Address) ([]byte, error) {
	code := statedb.GetCode(addr)
	if code == nil {
		return nil, fmt.Errorf("missing wasm at address %v", addr)
	}
	if !vm.IsPolyglotProgram(code) {
		return nil, fmt.Errorf("code at address %v is not a Polyglot WASM program", addr)
	}
	// Trim polyglot prefix bytes.
	wasm, err := vm.StripPolyglotPrefix(code)
	if err != nil {
		return nil, err
	}
	return arbcompress.Decompress(wasm, MaxWASMSize)
}
