// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package programs

import (
	"encoding/binary"
	"fmt"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/common/math"
	"github.com/ethereum/go-ethereum/core/state"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/log"
	"github.com/offchainlabs/nitro/arbcompress"
	"github.com/offchainlabs/nitro/arbos/storage"
	"github.com/offchainlabs/nitro/arbos/util"
	"github.com/offchainlabs/nitro/util/arbmath"
)

const (
	ByteOffsetVersion       = 0
	ByteOffsetInkPrice      = 4
	ByteOffsetWasmMaxDepth  = 12
	ByteOffsetWasmHostioInk = 16
	MaxWasmSize             = 64 * 1024
)

type Settings struct {
	Version       uint32
	InkPrice      arbmath.UBips
	WasmMaxDepth  uint32
	WasmHostioInk uint64
}

func (s *Settings) serialize() []byte {
	data := make([]byte, 24)
	binary.BigEndian.PutUint32(data[ByteOffsetVersion:], s.Version)
	binary.BigEndian.PutUint64(data[ByteOffsetInkPrice:], uint64(s.InkPrice))
	binary.BigEndian.PutUint32(data[ByteOffsetWasmMaxDepth:], s.WasmMaxDepth)
	binary.BigEndian.PutUint64(data[ByteOffsetWasmHostioInk:], s.WasmHostioInk)

	return data
}

func deserializeSettings(data []byte) *Settings {
	var settings Settings
	settings.Version = binary.BigEndian.Uint32(data[ByteOffsetVersion:])
	settings.InkPrice = arbmath.UBips(binary.BigEndian.Uint64(data[ByteOffsetInkPrice:]))
	settings.WasmMaxDepth = binary.BigEndian.Uint32(data[ByteOffsetWasmMaxDepth:])
	settings.WasmHostioInk = binary.BigEndian.Uint64(data[ByteOffsetWasmHostioInk:])

	return &settings
}

type Programs struct {
	backingStorage  *storage.Storage
	machineVersions *storage.Storage
}

var machineVersionsKey = []byte{0}

var ProgramNotCompiledError func() error
var ProgramOutOfDateError func(version uint32) error
var ProgramUpToDateError func() error

func Initialize(sto *storage.Storage) error {
	settings := Settings{
		Version:       1,
		InkPrice:      1,
		WasmMaxDepth:  math.MaxUint32,
		WasmHostioInk: 0,
	}
	return sto.SetBytes(settings.serialize())
}

func Open(sto *storage.Storage) *Programs {
	return &Programs{
		backingStorage:  sto,
		machineVersions: sto.OpenSubStorage(machineVersionsKey),
	}
}

func (p Programs) Settings() (*Settings, error) {
	bytes, err := p.backingStorage.GetBytes()
	if err != nil {
		return nil, err
	}
	return deserializeSettings(bytes), nil
}

func (p Programs) SaveSettings(settings *Settings) error {
	return p.backingStorage.SetBytes(settings.serialize())
}

func (p Programs) StylusVersion() (uint32, error) {
	settings, err := p.Settings()
	if err != nil {
		return 0, err
	}
	return settings.Version, nil
}

func (p Programs) InkPrice() (arbmath.UBips, error) {
	settings, err := p.Settings()
	if err != nil {
		return 0, err
	}
	return settings.InkPrice, nil
}

func (p Programs) SetInkPrice(price arbmath.UBips) error {
	settings, err := p.Settings()
	if err != nil {
		return err
	}
	settings.InkPrice = price
	return p.SaveSettings(settings)
}

func (p Programs) WasmMaxDepth() (uint32, error) {
	settings, err := p.Settings()
	if err != nil {
		return 0, err
	}
	return settings.WasmMaxDepth, nil
}

func (p Programs) SetWasmMaxDepth(depth uint32) error {
	settings, err := p.Settings()
	if err != nil {
		return err
	}
	settings.WasmMaxDepth = depth
	return p.SaveSettings(settings)
}

func (p Programs) WasmHostioInk() (uint64, error) {
	settings, err := p.Settings()
	if err != nil {
		return 0, err
	}
	return settings.WasmHostioInk, nil
}

func (p Programs) SetWasmHostioInk(cost uint64) error {
	settings, err := p.Settings()
	if err != nil {
		return err
	}
	settings.WasmHostioInk = cost
	return p.SaveSettings(settings)
}

func (p Programs) ProgramVersion(program common.Address) (uint32, error) {
	settings, err := p.Settings()
	if err != nil {
		return 0, err
	}
	return settings.Version, nil
}

func (p Programs) CompileProgram(statedb vm.StateDB, program common.Address, debugMode bool) (uint32, error) {
	version, err := p.StylusVersion()
	if err != nil {
		return 0, err
	}
	latest, err := p.machineVersions.GetUint32(program.Hash())
	if err != nil {
		return 0, err
	}
	if latest >= version {
		return 0, ProgramUpToDateError()
	}

	wasm, err := getWasm(statedb, program)
	if err != nil {
		return 0, err
	}
	if err := compileUserWasm(statedb, program, wasm, version, debugMode); err != nil {
		return 0, err
	}
	return version, p.machineVersions.SetUint32(program.Hash(), version)
}

func (p Programs) CallProgram(
	scope *vm.ScopeContext,
	statedb vm.StateDB,
	interpreter *vm.EVMInterpreter,
	tracingInfo *util.TracingInfo,
	calldata []byte,
) ([]byte, error) {
	stylusVersion, err := p.StylusVersion()
	if err != nil {
		return nil, err
	}
	programVersion, err := p.machineVersions.GetUint32(scope.Contract.Address().Hash())
	if err != nil {
		return nil, err
	}
	if programVersion == 0 {
		return nil, ProgramNotCompiledError()
	}
	if programVersion != stylusVersion {
		return nil, ProgramOutOfDateError(programVersion)
	}
	params, err := p.goParams(programVersion, interpreter.Evm().ChainConfig().DebugMode())
	if err != nil {
		return nil, err
	}
	evm := interpreter.Evm()
	evmData := &evmData{
		origin: evm.TxContext.Origin,
	}
	return callUserWasm(scope, statedb, interpreter, tracingInfo, calldata, evmData, params)
}

func getWasm(statedb vm.StateDB, program common.Address) ([]byte, error) {
	prefixedWasm := statedb.GetCode(program)
	if prefixedWasm == nil {
		return nil, fmt.Errorf("missing wasm at address %v", program)
	}
	wasm, err := state.StripStylusPrefix(prefixedWasm)
	if err != nil {
		return nil, err
	}
	return arbcompress.Decompress(wasm, MaxWasmSize)
}

type goParams struct {
	version   uint32
	maxDepth  uint32
	inkPrice  uint64
	hostioInk uint64
	debugMode uint32
}

func (p Programs) goParams(version uint32, debug bool) (*goParams, error) {
	settings, err := p.Settings()
	if err != nil {
		return nil, err
	}
	config := &goParams{
		version:   settings.Version,
		maxDepth:  settings.WasmMaxDepth,
		inkPrice:  settings.InkPrice.Uint64(),
		hostioInk: settings.WasmHostioInk,
	}
	if debug {
		config.debugMode = 1
	}
	return config, nil
}

type evmData struct {
	origin common.Address
}

type userStatus uint8

const (
	userSuccess userStatus = iota
	userRevert
	userFailure
	userOutOfGas
	userOutOfStack
)

func (status userStatus) output(data []byte) ([]byte, error) {
	switch status {
	case userSuccess:
		return data, nil
	case userRevert:
		return data, vm.ErrExecutionReverted
	case userFailure:
		return nil, vm.ErrExecutionReverted
	case userOutOfGas:
		return nil, vm.ErrOutOfGas
	case userOutOfStack:
		return nil, vm.ErrDepth
	default:
		log.Error("program errored with unknown status", "status", status, "data", common.Bytes2Hex(data))
		return nil, vm.ErrExecutionReverted
	}
}
