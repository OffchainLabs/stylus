// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package programs

import (
	"errors"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/offchainlabs/nitro/arbos/storage"
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

func (p Programs) AddProgram(statedb vm.StateDB, wasm []byte) common.Hash {
	hash := crypto.Keccak256Hash(wasm)
	polyAddProgram(statedb, hash, wasm)
	return hash
}

func (p Programs) CompileProgram(statedb vm.StateDB, wasm_hash common.Hash) error {
	err := polyCompile(statedb, wasm_hash)
	if err != nil {
		return err
	}
	return p.machineVersions.SetUint64(wasm_hash, 1)
}

func (p Programs) CallProgram(
	statedb vm.StateDB,
	wasm_hash common.Hash,
	calldata []byte,
	gas uint64,
) (uint64, uint64, []byte, error) {
	version, err := p.machineVersions.GetUint64(wasm_hash)
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
	burnt, status, output := polyExecute(statedb, wasm_hash, calldata, gas, gasPrice)
	return burnt, status, output, nil
}
