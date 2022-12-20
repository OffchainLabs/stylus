// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

//go:build !js
// +build !js

package programs

//#cgo CFLAGS: -g -Wall
//#cgo LDFLAGS: ${SRCDIR}/../../target/lib/libpolyglot.a -ldl -lm
//#include <stdint.h>
// extern size_t polyglot_compile(const uint8_t * wasm, size_t len, uint8_t ** out, size_t * out_len, size_t * out_cap);
// size_t polyglot_call(
//    const uint8_t * module, size_t module_len,
//    const uint8_t * input, size_t input_len,
//    uint8_t ** output, size_t * output_len, size_t * output_cap,
//    uint64_t * gas, uint64_t gas_price
// );
// extern void polyglot_free(uint8_t * data, size_t out_len, size_t out_cap);
import "C"
import (
	"errors"
	"unsafe"

	"fmt"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/state"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/ethdb"
	"github.com/ethereum/go-ethereum/log"
	"github.com/offchainlabs/nitro/arbutil"
)

type u32 = uint32
type u64 = uint64

func polyCompile(statedb vm.StateDB, arbDB ethdb.Database, program common.Address, wasm []byte) error {

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
	defer polyFree(outptr, outlen, outcap)

	output := arbutil.PointerToSlice((*byte)(outptr), outlen)
	if status != 0 {
		return errors.New(string(output))
	}
	addPolyMachine(arbDB, 1, program, output)
	fmt.Printf("Added machine: %#x and %#x\n", program, output[:24])
	return nil
}

func polyCall(statedb vm.StateDB, arbDB ethdb.Database, program common.Address, calldata []byte, gas, gas_price u64) (u64, u32, []byte) {

	if db, ok := statedb.(*state.StateDB); ok {
		db.RecordProgram(program)
	}

	machine, err := getPolyMachine(arbDB, 1, program)
	if err != nil {
		log.Crit("machine does not exist")
	}
	fmt.Printf("Loaded machine: %#x\n", machine[:24])

	// call into rust with C-signature
	//     size_t polyglot_call(
	//         const uint8_t * module, size_t module_len,
	//         const uint8_t * input, size_t input_len,
	//         uint8_t * output, size_t output_len, size_t output_cap,
	//         uint64_t gas, uint64_t gas_price
	//     );
	var outptr *C.uint8_t
	outlen := 0
	outcap := 0
	status := C.polyglot_call(
		(*C.uint8_t)(arbutil.SliceToPointer(machine)),
		C.size_t(len(machine)),
		(*C.uint8_t)(arbutil.SliceToPointer(calldata)),
		C.size_t(len(calldata)),
		(**C.uint8_t)(&outptr),
		(*C.size_t)(unsafe.Pointer(&outlen)),
		(*C.size_t)(unsafe.Pointer(&outcap)),
		(*C.uint64_t)(unsafe.Pointer(&gas)),
		C.uint64_t(gas_price), // nolint:gocritic
	)
	defer polyFree(outptr, outlen, outcap)

	output := arbutil.PointerToSlice((*byte)(outptr), outlen)
	return gas, u32(status), output
}

func polyFree(ptr *C.uint8_t, len, cap int) {
	// free the rust-side return data by calling
	//     void polyglot_free(uint8_t * data, size_t out_len, size_t out_cap);
	//
	C.polyglot_free(ptr, C.size_t(len), C.size_t(cap))
}

func addPolyMachine(writer ethdb.KeyValueWriter, version uint64, program common.Address, output []byte) {
	prefix := []byte("polyglot-machines")
	key := append(prefix, program[:]...)
	writer.Put(key, output)
}

func getPolyMachine(writer ethdb.KeyValueReader, version uint64, program common.Address) ([]byte, error) {
	prefix := []byte("polyglot-machines")
	key := append(prefix, program[:]...)
	return writer.Get(key)
}
