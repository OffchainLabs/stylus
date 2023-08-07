// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

package programs

import (
	"errors"
	"math/big"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/log"
	"github.com/ethereum/go-ethereum/params"
	"github.com/holiman/uint256"
	"github.com/offchainlabs/nitro/arbos/util"
	"github.com/offchainlabs/nitro/util/arbmath"
)

type getBytes32Type func(key common.Hash) (value common.Hash, cost uint64)
type setBytes32Type func(key, value common.Hash) (cost uint64, err error)
type contractCallType func(
	contract common.Address, calldata []byte, gas uint64, value *big.Int) (
	retdata_len uint32, cost uint64, err error,
)
type delegateCallType func(
	contract common.Address, calldata []byte, gas uint64) (
	retdata_len uint32, cost uint64, err error,
)
type staticCallType func(
	contract common.Address, calldata []byte, gas uint64) (
	retdata_len uint32, cost uint64, err error,
)
type create1Type func(
	code []byte, endowment *big.Int, gas uint64) (
	addr common.Address, retdata_len uint32, cost uint64, err error,
)
type create2Type func(
	code []byte, salt, endowment *big.Int, gas uint64) (
	addr common.Address, retdata_len uint32, cost uint64, err error,
)
type getReturnDataType func(offset uint32, size uint32) []byte
type emitLogType func(data []byte, topics uint32) error
type reportHostioType func(opcode uint32, gas uint64, cost uint64) error
type reportHostioAdvancedType func(opcode uint32, data []byte, offset uint32, size uint32, gas uint64, cost uint64) error
type accountBalanceType func(address common.Address) (value common.Hash, cost uint64)
type accountCodehashType func(address common.Address) (value common.Hash, cost uint64)
type addPagesType func(pages uint16) (cost uint64)

type goClosures struct {
	getBytes32           getBytes32Type
	setBytes32           setBytes32Type
	contractCall         contractCallType
	delegateCall         delegateCallType
	staticCall           staticCallType
	create1              create1Type
	create2              create2Type
	getReturnData        getReturnDataType
	emitLog              emitLogType
	reportHostio         reportHostioType
	reportHostioAdvanced reportHostioAdvancedType
	accountBalance       accountBalanceType
	accountCodeHash      accountCodehashType
	addPages             addPagesType
}

func newApiClosures(
	interpreter *vm.EVMInterpreter,
	tracingInfo *util.TracingInfo,
	scope *vm.ScopeContext,
	memoryModel *MemoryModel,
) *goClosures {
	contract := scope.Contract
	actingAddress := contract.Address() // not necessarily WASM
	readOnly := interpreter.ReadOnly()
	evm := interpreter.Evm()
	depth := evm.Depth()
	db := evm.StateDB

	getBytes32 := func(key common.Hash) (common.Hash, uint64) {
		if tracingInfo != nil {
			tracingInfo.RecordStorageGet(key)
		}
		cost := vm.WasmStateLoadCost(db, actingAddress, key)
		return db.GetState(actingAddress, key), cost
	}
	setBytes32 := func(key, value common.Hash) (uint64, error) {
		if tracingInfo != nil {
			tracingInfo.RecordStorageSet(key, value)
		}
		if readOnly {
			return 0, vm.ErrWriteProtection
		}
		cost := vm.WasmStateStoreCost(db, actingAddress, key, value)
		db.SetState(actingAddress, key, value)
		return cost, nil
	}
	doCall := func(
		contract common.Address, opcode vm.OpCode, input []byte, gas uint64, value *big.Int,
	) (uint32, uint64, error) {
		// This closure can perform each kind of contract call based on the opcode passed in.
		// The implementation for each should match that of the EVM.
		//
		// Note that while the Yellow Paper is authoritative, the following go-ethereum
		// functions provide corresponding implementations in the vm package.
		//     - operations_acl.go makeCallVariantGasCallEIP2929()
		//     - gas_table.go      gasCall() gasDelegateCall() gasStaticCall()
		//     - instructions.go   opCall()  opDelegateCall()  opStaticCall()
		//

		// read-only calls are not payable (opCall)
		if readOnly && value.Sign() != 0 {
			return 0, 0, vm.ErrWriteProtection
		}

		startGas := gas

		// computes makeCallVariantGasCallEIP2929 and gasCall/gasDelegateCall/gasStaticCall
		baseCost, err := vm.WasmCallCost(db, contract, value, startGas)
		if err != nil {
			return 0, gas, err
		}
		gas -= baseCost

		// apply the 63/64ths rule
		one64th := gas / 64
		gas -= one64th

		// Tracing: emit the call (value transfer is done later in evm.Call)
		if tracingInfo != nil {
			tracingInfo.Tracer.CaptureState(0, opcode, startGas, baseCost+gas, scope, []byte{}, depth, nil)
		}

		// EVM rule: calls that pay get a stipend (opCall)
		if value.Sign() != 0 {
			gas = arbmath.SaturatingUAdd(gas, params.CallStipend)
		}

		var ret []byte
		var returnGas uint64

		switch opcode {
		case vm.CALL:
			ret, returnGas, err = evm.Call(scope.Contract, contract, input, gas, value)
		case vm.DELEGATECALL:
			ret, returnGas, err = evm.DelegateCall(scope.Contract, contract, input, gas)
		case vm.STATICCALL:
			ret, returnGas, err = evm.StaticCall(scope.Contract, contract, input, gas)
		default:
			log.Crit("unsupported call type", "opcode", opcode)
		}

		interpreter.SetReturnData(ret)
		cost := arbmath.SaturatingUSub(startGas, returnGas+one64th) // user gets 1/64th back
		return uint32(len(ret)), cost, err
	}
	contractCall := func(contract common.Address, input []byte, gas uint64, value *big.Int) (uint32, uint64, error) {
		return doCall(contract, vm.CALL, input, gas, value)
	}
	delegateCall := func(contract common.Address, input []byte, gas uint64) (uint32, uint64, error) {
		return doCall(contract, vm.DELEGATECALL, input, gas, common.Big0)
	}
	staticCall := func(contract common.Address, input []byte, gas uint64) (uint32, uint64, error) {
		return doCall(contract, vm.STATICCALL, input, gas, common.Big0)
	}
	create := func(code []byte, endowment, salt *big.Int, gas uint64) (common.Address, uint32, uint64, error) {
		// This closure can perform both kinds of contract creation based on the salt passed in.
		// The implementation for each should match that of the EVM.
		//
		// Note that while the Yellow Paper is authoritative, the following go-ethereum
		// functions provide corresponding implementations in the vm package.
		//     - instructions.go opCreate() opCreate2()
		//     - gas_table.go    gasCreate() gasCreate2()
		//

		opcode := vm.CREATE
		if salt != nil {
			opcode = vm.CREATE2
		}
		zeroAddr := common.Address{}
		startGas := gas

		if readOnly {
			return zeroAddr, 0, 0, vm.ErrWriteProtection
		}

		// pay for static and dynamic costs (gasCreate and gasCreate2)
		baseCost := params.CreateGas
		if opcode == vm.CREATE2 {
			keccakWords := arbmath.WordsForBytes(uint64(len(code)))
			keccakCost := arbmath.SaturatingUMul(params.Keccak256WordGas, keccakWords)
			baseCost = arbmath.SaturatingUAdd(baseCost, keccakCost)
		}
		if gas < baseCost {
			return zeroAddr, 0, gas, vm.ErrOutOfGas
		}
		gas -= baseCost

		// apply the 63/64ths rule
		one64th := gas / 64
		gas -= one64th

		// Tracing: emit the create
		if tracingInfo != nil {
			tracingInfo.Tracer.CaptureState(0, opcode, startGas, baseCost+gas, scope, []byte{}, depth, nil)
		}

		var res []byte
		var addr common.Address // zero on failure
		var returnGas uint64
		var suberr error

		if opcode == vm.CREATE {
			res, addr, returnGas, suberr = evm.Create(contract, code, gas, endowment)
		} else {
			salt256, _ := uint256.FromBig(salt)
			res, addr, returnGas, suberr = evm.Create2(contract, code, gas, endowment, salt256)
		}
		if suberr != nil {
			addr = zeroAddr
		}
		if !errors.Is(vm.ErrExecutionReverted, suberr) {
			res = nil // returnData is only provided in the revert case (opCreate)
		}
		interpreter.SetReturnData(res)
		cost := arbmath.SaturatingUSub(startGas, returnGas+one64th) // user gets 1/64th back
		return addr, uint32(len(res)), cost, nil
	}
	create1 := func(code []byte, endowment *big.Int, gas uint64) (common.Address, uint32, uint64, error) {
		return create(code, endowment, nil, gas)
	}
	create2 := func(code []byte, endowment, salt *big.Int, gas uint64) (common.Address, uint32, uint64, error) {
		return create(code, endowment, salt, gas)
	}
	getReturnData := func(offset uint32, size uint32) []byte {
		data := interpreter.GetReturnData(int(offset), int(size))
		if data == nil {
			return []byte{}
		}
		return data
	}
	extractLogData := func(data []byte, topics uint32) ([]common.Hash, []byte, error) {
		if len(data) == 0 {
			return []common.Hash{}, []byte{}, nil
		}
		if len(data) < int(32*topics) {
			return nil, nil, errors.New("emit log data too small")
		}
		hashes := make([]common.Hash, topics)
		for i := uint32(0); i < topics; i++ {
			hashes[i] = common.BytesToHash(data[:(i+1)*32])
		}
		if len(data) < int(32*topics)+1 {
			return hashes, []byte{}, nil
		}
		logData := data[32*topics:]

		return hashes, logData, nil
	}
	emitLog := func(data []byte, topics uint32) error {
		if readOnly {
			return vm.ErrWriteProtection
		}
		hashes, logData, err := extractLogData(data, topics)
		if err != nil {
			return err
		}
		event := &types.Log{
			Address:     actingAddress,
			Topics:      hashes,
			Data:        logData,
			BlockNumber: evm.Context.BlockNumber.Uint64(),
			// Geth will set other fields
		}
		db.AddLog(event)
		return nil
	}
	reportHostio := func(rawOpcode uint32, gas uint64, cost uint64) error {
		opcode := vm.OpCode(rawOpcode)
		if tracingInfo != nil {
			switch opcode {
			case vm.CALLDATALOAD:
				tracingInfo.RecordCallDataLoad(gas, cost)
			default:
				tracingInfo.RecordWithNoFields(opcode, gas, cost)
			}
		}

		return nil
	}
	reportHostioAdvanced := func(rawOpcode uint32, data []byte, offset uint32, size uint32, gas uint64, cost uint64) error {
		opcode := vm.OpCode(rawOpcode)
		if tracingInfo != nil {
			switch opcode {
			case vm.RETURNDATACOPY:
				tracingInfo.RecordWithPartialFields(opcode, offset, size, gas, cost)
			case vm.KECCAK256:
				tracingInfo.RecordWithDataFields(opcode, data, gas, cost)
			case vm.ADDRESS, vm.EXTCODEHASH:
				tracingInfo.RecordWithAddress(opcode, common.BytesToAddress(data), gas, cost)
			case vm.LOG0:
				hashes, logData, err := extractLogData(data, size)
				if err != nil {
					return err
				}
				tracingInfo.RecordEmitLog(logData, hashes, gas, cost)
			default:
				return errors.New("trace: unrecognized opcode")
			}
		}

		return nil
	}
	accountBalance := func(address common.Address) (common.Hash, uint64) {
		cost := vm.WasmAccountTouchCost(evm.StateDB, address)
		balance := evm.StateDB.GetBalance(address)
		return common.BigToHash(balance), cost
	}
	accountCodehash := func(address common.Address) (common.Hash, uint64) {
		cost := vm.WasmAccountTouchCost(evm.StateDB, address)
		if !evm.StateDB.Empty(address) {
			return evm.StateDB.GetCodeHash(address), cost
		}
		return common.Hash{}, cost
	}
	addPages := func(pages uint16) uint64 {
		open, ever := db.AddStylusPages(pages)
		return memoryModel.GasCost(pages, open, ever)
	}

	return &goClosures{
		getBytes32:           getBytes32,
		setBytes32:           setBytes32,
		contractCall:         contractCall,
		delegateCall:         delegateCall,
		staticCall:           staticCall,
		create1:              create1,
		create2:              create2,
		getReturnData:        getReturnData,
		emitLog:              emitLog,
		reportHostio:         reportHostio,
		reportHostioAdvanced: reportHostioAdvanced,
		accountBalance:       accountBalance,
		accountCodeHash:      accountCodehash,
		addPages:             addPages,
	}
}
