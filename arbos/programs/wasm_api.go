// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

//go:build js
// +build js

package programs

import (
	"fmt"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/offchainlabs/nitro/arbos/util"
	"github.com/offchainlabs/nitro/util/arbmath"
	"math/big"
	"syscall/js"
)

type apiWrapper struct {
	getBytes32           js.Func
	setBytes32           js.Func
	contractCall         js.Func
	delegateCall         js.Func
	staticCall           js.Func
	create1              js.Func
	create2              js.Func
	getReturnData        js.Func
	emitLog              js.Func
	reportHostio         js.Func
	reportHostioAdvanced js.Func
	addressBalance       js.Func
	addressCodeHash      js.Func
	addPages             js.Func
	funcs                []byte
}

func newApi(
	interpreter *vm.EVMInterpreter,
	tracingInfo *util.TracingInfo,
	scope *vm.ScopeContext,
	memoryModel *MemoryModel,
) *apiWrapper {
	closures := newApiClosures(interpreter, tracingInfo, scope, memoryModel)
	global := js.Global()
	uint8Array := global.Get("Uint8Array")

	const (
		preU16 = iota
		preU32
		preU64
		preBytes
		preBytes20
		preBytes32
		preString
		preNil
	)

	jsRead := func(value js.Value, kind u8) []u8 {
		length := value.Length()
		data := make([]u8, length)
		js.CopyBytesToGo(data, value)
		if data[0] != kind {
			panic(fmt.Sprintf("not a %v", kind))
		}
		return data[1:]
	}
	jsU16 := func(value js.Value) u16 {
		return arbmath.BytesToUint16(jsRead(value, preU16))
	}
	jsU32 := func(value js.Value) u32 {
		return arbmath.BytesToUint32(jsRead(value, preU32))
	}
	jsU64 := func(value js.Value) u64 {
		return arbmath.BytesToUint(jsRead(value, preU64))
	}
	jsBytes := func(value js.Value) []u8 {
		return jsRead(value, preBytes)
	}
	jsAddress := func(value js.Value) common.Address {
		return common.BytesToAddress(jsRead(value, preBytes20))
	}
	jsHash := func(value js.Value) common.Hash {
		return common.BytesToHash(jsRead(value, preBytes32))
	}
	jsBig := func(value js.Value) *big.Int {
		return jsHash(value).Big()
	}

	toJs := func(prefix u8, data []byte) js.Value {
		value := append([]byte{prefix}, data...)
		array := uint8Array.New(len(value))
		js.CopyBytesToJS(array, value)
		return array
	}
	write := func(stylus js.Value, results ...any) any {
		array := make([]interface{}, 0)
		for _, result := range results {
			var value js.Value
			switch result := result.(type) {
			case uint16:
				value = toJs(preU16, arbmath.Uint16ToBytes(result))
			case uint32:
				value = toJs(preU32, arbmath.Uint32ToBytes(result))
			case uint64:
				value = toJs(preU64, arbmath.UintToBytes(result))
			case []u8:
				value = toJs(preBytes, result[:])
			case common.Address:
				value = toJs(preBytes20, result[:])
			case common.Hash:
				value = toJs(preBytes32, result[:])
			case error:
				if result == nil {
					value = toJs(preNil, []byte{})
				} else {
					value = toJs(preString, []byte(result.Error()))
				}
			case nil:
				value = toJs(preNil, []byte{})
			default:
				panic("Unable to coerce value")
			}
			array = append(array, value)
		}
		stylus.Set("result", js.ValueOf(array))
		return nil
	}
	maybe := func(value interface{}, err error) interface{} {
		if err != nil {
			return err
		}
		return value
	}

	getBytes32 := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		key := jsHash(args[0])
		value, cost := closures.getBytes32(key)
		return write(stylus, value, cost)
	})
	setBytes32 := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		key := jsHash(args[0])
		value := jsHash(args[1])
		cost, err := closures.setBytes32(key, value)
		return write(stylus, maybe(cost, err))
	})
	contractCall := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		contract := jsAddress(args[0])
		input := jsBytes(args[1])
		gas := jsU64(args[2])
		value := jsBig(args[3])
		len, cost, status := closures.contractCall(contract, input, gas, value)
		return write(stylus, len, cost, status)
	})
	delegateCall := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		contract := jsAddress(args[0])
		input := jsBytes(args[1])
		gas := jsU64(args[2])
		len, cost, status := closures.delegateCall(contract, input, gas)
		return write(stylus, len, cost, status)
	})
	staticCall := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		contract := jsAddress(args[0])
		input := jsBytes(args[1])
		gas := jsU64(args[2])
		len, cost, status := closures.staticCall(contract, input, gas)
		return write(stylus, len, cost, status)
	})
	create1 := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		code := jsBytes(args[0])
		endowment := jsBig(args[1])
		gas := jsU64(args[2])
		addr, len, cost, err := closures.create1(code, endowment, gas)
		return write(stylus, maybe(addr, err), len, cost)
	})
	create2 := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		code := jsBytes(args[0])
		endowment := jsBig(args[1])
		salt := jsBig(args[2])
		gas := jsU64(args[3])
		addr, len, cost, err := closures.create2(code, endowment, salt, gas)
		return write(stylus, maybe(addr, err), len, cost)
	})
	getReturnData := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		offset := jsU32(args[0])
		size := jsU32(args[1])
		data := closures.getReturnData(offset, size)
		return write(stylus, data)
	})
	emitLog := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		data := jsBytes(args[0])
		topics := jsU32(args[1])
		err := closures.emitLog(data, topics)
		return write(stylus, err)
	})
	reportHostio := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		opcode := jsU32(args[0])
		gas := jsU64(args[1])
		cost := jsU64(args[2])
		err := closures.reportHostio(opcode, gas, cost)
		return write(stylus, err)
	})
	reportHostioAdvanced := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		opcode := jsU32(args[0])
		data := jsBytes(args[1])
		offset := jsU32(args[2])
		size := jsU32(args[3])
		gas := jsU64(args[4])
		cost := jsU64(args[5])
		err := closures.reportHostioAdvanced(opcode, data, offset, size, gas, cost)
		return write(stylus, err)
	})
	addressBalance := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		address := jsAddress(args[0])
		value, cost := closures.accountBalance(address)
		return write(stylus, value, cost)
	})
	addressCodeHash := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		address := jsAddress(args[0])
		value, cost := closures.accountCodeHash(address)
		return write(stylus, value, cost)
	})
	addPages := js.FuncOf(func(stylus js.Value, args []js.Value) any {
		pages := jsU16(args[0])
		cost := closures.addPages(pages)
		return write(stylus, cost)
	})

	ids := make([]byte, 0, 14*4)
	funcs := js.Global().Get("stylus").Call("setCallbacks",
		getBytes32, setBytes32, contractCall, delegateCall,
		staticCall, create1, create2, getReturnData, emitLog,
		reportHostio, reportHostioAdvanced, addressBalance,
		addressCodeHash, addPages,
	)
	for i := 0; i < funcs.Length(); i++ {
		ids = append(ids, arbmath.Uint32ToBytes(u32(funcs.Index(i).Int()))...)
	}
	return &apiWrapper{
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
		addressBalance:       addressBalance,
		addressCodeHash:      addressCodeHash,
		addPages:             addPages,
		funcs:                ids,
	}
}

func (api *apiWrapper) drop() {
	api.getBytes32.Release()
	api.setBytes32.Release()
	api.contractCall.Release()
	api.delegateCall.Release()
	api.staticCall.Release()
	api.create1.Release()
	api.create2.Release()
	api.getReturnData.Release()
	api.emitLog.Release()
	api.reportHostio.Release()
	api.reportHostioAdvanced.Release()
	api.addressBalance.Release()
	api.addressCodeHash.Release()
	api.addPages.Release()
}
