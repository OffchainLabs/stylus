// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

//go:build !js
// +build !js

package programs

//#cgo CFLAGS: -g -Wall
//#cgo LDFLAGS: ${SRCDIR}/../../arbitrator/target/release/libpolyglot.a -ldl -lm
//#include <stdint.h>
// extern size_t polyglot_compile(const uint8_t * wasm, size_t len, uint8_t ** out, size_t * out_len, size_t * out_cap);
// size_t polyglot_call(
//    const uint8_t * module, size_t module_len,
//    const uint8_t * input, size_t input_len,
//    uint8_t ** output, size_t * output_len, size_t * output_cap,
//    uint64_t gas, uint64_t * gas_burnt, uint64_t gas_price
// );
// extern void polyglot_free(uint8_t * data, size_t out_len, size_t out_cap);
import "C"
import (
	"errors"
	"unsafe"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/log"
	"github.com/offchainlabs/nitro/arbutil"
)

func polyAddProgram(statedb vm.StateDB, hash common.Hash, wasm []byte) {
	statedb.AddPolyProgram(hash, wasm)
}

func polyCompile(statedb vm.StateDB, wasm_hash common.Hash) error {
	wasm := statedb.GetPolyProgram(wasm_hash)

	// call into rust with C-signature
	//     size_t polyglot_compile(uint8_t * wasm, size_t len, uint8_t ** out, size_t * out_len, size_t * out_cap)
	//
	var outptr *C.uint8_t
	outlen := 0
	outcap := 0
	status := C.polyglot_compile(
		(*C.uint8_t)(arbutil.SliceToPointer(wasm)),
		C.size_t(len(wasm)),
		(**C.uint8_t)(&outptr),
		(*C.size_t)(unsafe.Pointer(&outlen)),
		(*C.size_t)(unsafe.Pointer(&outcap)), // nolint:gocritic
	)

	defer func() {
		// free the rust-side return data by calling
		//     void polyglot_free(uint8_t * data, size_t out_len, size_t out_cap);
		//
		C.polyglot_free(outptr, C.size_t(outlen), C.size_t(outcap))
	}()

	output := make([]byte, outlen)
	source := unsafe.Slice((*byte)(outptr), outlen)
	copy(output, source)

	if status != 0 {
		return errors.New(string(output))
	}
	statedb.AddPolyMachine(1, wasm_hash, output)
	return nil
}

func polyExecute(
	statedb vm.StateDB, wasm_hash common.Hash, calldata []byte, gas uint64, gas_price uint64,
) (uint64, uint64, []byte) {

	machine, err := statedb.GetPolyMachine(1, wasm_hash)
	if err != nil {
		log.Crit("machine does not exist")
	}

	// call into rust with C-signature
	//     size_t polyglot_call(
	//         const uint8_t * module, size_t module_len,
	//         const uint8_t * input, size_t input_len,
	//         uint8_t * output, size_t output_len, size_t output_cap,
	//         uint64_t gas, uint64_t * gas_left, uint64_t gas_price
	//     );
	var outptr *C.uint8_t
	outlen := 0
	outcap := 0
	gas_burnt := uint64(0)
	status := C.polyglot_call(
		(*C.uint8_t)(arbutil.SliceToPointer(machine)),
		C.size_t(len(machine)),
		(*C.uint8_t)(arbutil.SliceToPointer(calldata)),
		C.size_t(len(calldata)),
		(**C.uint8_t)(&outptr),
		(*C.size_t)(unsafe.Pointer(&outlen)),
		(*C.size_t)(unsafe.Pointer(&outcap)),
		C.uint64_t(gas),
		(*C.uint64_t)(unsafe.Pointer(&gas_burnt)),
		C.uint64_t(gas_price), // nolint:gocritic
	)

	defer func() {
		// free the rust-side return data by calling
		//     void polyglot_free(uint8_t * data, size_t out_len, size_t out_cap);
		//
		C.polyglot_free(outptr, C.size_t(outlen), C.size_t(outcap))
	}()

	output := make([]byte, outlen)
	source := unsafe.Slice((*byte)(outptr), outlen)
	copy(output, source)

	return gas_burnt, uint64(status), output
}
