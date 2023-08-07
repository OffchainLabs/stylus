// Copyright 2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

//go:build !js
// +build !js

package programs

/*
#cgo CFLAGS: -g -Wall -I../../target/include/
#cgo LDFLAGS: ${SRCDIR}/../../target/lib/libstylus.a -ldl -lm
#include "arbitrator.h"

typedef uint16_t u16;
typedef uint32_t u32;
typedef uint64_t u64;
typedef size_t usize;

Bytes32 getBytes32Impl(usize api, Bytes32 key, u64 * cost);
Bytes32 getBytes32Wrap(usize api, Bytes32 key, u64 * cost) {
    return getBytes32Impl(api, key, cost);
}

EvmApiStatus setBytes32Impl(usize api, Bytes32 key, Bytes32 value, u64 * cost, RustVec * error);
EvmApiStatus setBytes32Wrap(usize api, Bytes32 key, Bytes32 value, u64 * cost, RustVec * error) {
    return setBytes32Impl(api, key, value, cost, error);
}

EvmApiStatus contractCallImpl(usize api, Bytes20 contract, RustVec * calldata, u64 * gas, Bytes32 value, u32 * len);
EvmApiStatus contractCallWrap(usize api, Bytes20 contract, RustVec * calldata, u64 * gas, Bytes32 value, u32 * len) {
    return contractCallImpl(api, contract, calldata, gas, value, len);
}

EvmApiStatus delegateCallImpl(usize api, Bytes20 contract, RustVec * calldata, u64 * gas, u32 * len);
EvmApiStatus delegateCallWrap(usize api, Bytes20 contract, RustVec * calldata, u64 * gas, u32 * len) {
    return delegateCallImpl(api, contract, calldata, gas, len);
}

EvmApiStatus staticCallImpl(usize api, Bytes20 contract, RustVec * calldata, u64 * gas, u32 * len);
EvmApiStatus staticCallWrap(usize api, Bytes20 contract, RustVec * calldata, u64 * gas, u32 * len) {
    return staticCallImpl(api, contract, calldata, gas, len);
}

EvmApiStatus create1Impl(usize api, RustVec * code, Bytes32 endowment, u64 * gas, u32 * len);
EvmApiStatus create1Wrap(usize api, RustVec * code, Bytes32 endowment, u64 * gas, u32 * len) {
    return create1Impl(api, code, endowment, gas, len);
}

EvmApiStatus create2Impl(usize api, RustVec * code, Bytes32 endowment, Bytes32 salt, u64 * gas, u32 * len);
EvmApiStatus create2Wrap(usize api, RustVec * code, Bytes32 endowment, Bytes32 salt, u64 * gas, u32 * len) {
    return create2Impl(api, code, endowment, salt, gas, len);
}

void getReturnDataImpl(usize api, RustVec * data, u32 offset, u32 size);
void getReturnDataWrap(usize api, RustVec * data, u32 offset, u32 size) {
    return getReturnDataImpl(api, data, offset, size);
}

EvmApiStatus emitLogImpl(usize api, RustVec * data, usize topics);
EvmApiStatus emitLogWrap(usize api, RustVec * data, usize topics) {
    return emitLogImpl(api, data, topics);
}

EvmApiStatus reportHostioImpl(usize api, u32 opcode, u32 gas, u32 cost);
EvmApiStatus reportHostioWrap(usize api, u32 opcode, u32 gas, u32 cost) {
    return reportHostioImpl(api, opcode, gas, cost);
}

EvmApiStatus reportHostioAdvancedImpl(usize api, u32 opcode, RustVec * data, u32 offset, u32 size, u32 gas, u32 cost);
EvmApiStatus reportHostioAdvancedWrap(usize api, u32 opcode, RustVec * data, u32 offset, u32 size, u32 gas, u32 cost) {
    return reportHostioAdvancedImpl(api, opcode, data, offset, size, gas, cost);
}

Bytes32 accountBalanceImpl(usize api, Bytes20 address, u64 * cost);
Bytes32 accountBalanceWrap(usize api, Bytes20 address, u64 * cost) {
    return accountBalanceImpl(api, address, cost);
}

Bytes32 accountCodeHashImpl(usize api, Bytes20 address, u64 * cost);
Bytes32 accountCodeHashWrap(usize api, Bytes20 address, u64 * cost) {
    return accountCodeHashImpl(api, address, cost);
}

u64 addPagesImpl(usize api, u16 pages);
u64 addPagesWrap(usize api, u16 pages) {
    return addPagesImpl(api, pages);
}
*/
import "C"
import (
	"sync"
	"sync/atomic"

	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/log"
	"github.com/offchainlabs/nitro/arbos/util"
)

var apiClosures sync.Map
var apiIds uintptr // atomic

func newApi(
	interpreter *vm.EVMInterpreter,
	tracingInfo *util.TracingInfo,
	scope *vm.ScopeContext,
	memoryModel *MemoryModel,
) (C.GoEvmApi, usize) {
	closures := newApiClosures(interpreter, tracingInfo, scope, memoryModel)
	apiId := atomic.AddUintptr(&apiIds, 1)
	apiClosures.Store(apiId, closures)
	id := usize(apiId)
	return C.GoEvmApi{
		get_bytes32:            (*[0]byte)(C.getBytes32Wrap),
		set_bytes32:            (*[0]byte)(C.setBytes32Wrap),
		contract_call:          (*[0]byte)(C.contractCallWrap),
		delegate_call:          (*[0]byte)(C.delegateCallWrap),
		static_call:            (*[0]byte)(C.staticCallWrap),
		create1:                (*[0]byte)(C.create1Wrap),
		create2:                (*[0]byte)(C.create2Wrap),
		get_return_data:        (*[0]byte)(C.getReturnDataWrap),
		emit_log:               (*[0]byte)(C.emitLogWrap),
		report_hostio:          (*[0]byte)(C.reportHostioWrap),
		report_hostio_advanced: (*[0]byte)(C.reportHostioAdvancedWrap),
		account_balance:        (*[0]byte)(C.accountBalanceWrap),
		account_codehash:       (*[0]byte)(C.accountCodeHashWrap),
		add_pages:              (*[0]byte)(C.addPagesWrap),
		id:                     id,
	}, id
}

func getApi(id usize) *goClosures {
	any, ok := apiClosures.Load(uintptr(id))
	if !ok {
		log.Crit("failed to load stylus Go API", "id", id)
	}
	closures, ok := any.(*goClosures)
	if !ok {
		log.Crit("wrong type for stylus Go API", "id", id)
	}
	return closures
}

func dropApi(id usize) {
	apiClosures.Delete(uintptr(id))
}
