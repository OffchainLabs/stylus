// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package programs

import (
	"errors"
	"fmt"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/ethdb"
	"github.com/offchainlabs/nitro/arbcompress"
	"github.com/offchainlabs/nitro/arbos/storage"
)

const (
	MaxWASMSize = 64 * 1024
	// PolyglotMachineVersion defines the version number for polyglot
	// machines stored in ArbDB.
	PolyglotVersion = 1
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

func (p Programs) CompileProgram(statedb vm.StateDB, arbDB ethdb.Database, addr common.Address) error {
	wasm, err := getWasm(statedb, addr)
	if err != nil {
		return err
	}
	if err := polyCompile(statedb, arbDB, addr, wasm); err != nil {
		return err
	}
	return p.machineVersions.SetUint64(addr.Hash(), 1)
}

func (p Programs) CallProgram(
	statedb vm.StateDB,
	arbDB ethdb.Database,
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
	gasLeft, status, output := polyCall(statedb, arbDB, program, calldata, gas(), gasPrice)
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
