// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package programs

import (
	"errors"
	"fmt"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/offchainlabs/nitro/arbcompress"
	"github.com/offchainlabs/nitro/arbos/storage"
)

const MaxWASMSize = 64 * 1024

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

func (p Programs) CompileProgram(statedb vm.StateDB, program common.Address) error {
	wasm, err := getWasm(statedb, program)
	if err != nil {
		return err
	}
	if err := polyCompile(statedb, program, wasm); err != nil {
		return err
	}
	return p.machineVersions.SetUint64(program.Hash(), 1)
}

func (p Programs) CallProgram(
	statedb vm.StateDB,
	program common.Address,
	calldata []byte,
	gas func() uint64,
) (uint64, uint64, []byte, error) {
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
	gasLeft, status, output := polyCall(statedb, program, calldata, gas(), gasPrice)
	return gasLeft, status, output, nil
}

func getWasm(statedb vm.StateDB, program common.Address) ([]byte, error) {
	wasm := statedb.GetCode(program)
	if wasm == nil {
		return nil, fmt.Errorf("missing wasm at address %v", program)
	}
	return arbcompress.Decompress(wasm, MaxWASMSize)
}
