// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package precompiles

import "errors"

type ArbWasm struct {
	Address addr // 0x71
}

// Compile a wasm program with the latest instrumentation
func (con ArbWasm) CompileProgram(c ctx, evm mech, program addr) (uint32, error) {
	return 0, errors.New("unimplemented")
}

// Calls a wasm program
// TODO: move into geth
func (con ArbWasm) CallProgram(c ctx, evm mech, program addr, data []byte) (uint32, []byte, error) {
	return 0, nil, errors.New("unimplemented")
}

// Gets the latest polyglot version
func (con ArbWasm) PolyglotVersion(c ctx, evm mech) (uint32, error) {
	return c.State.Programs().PolyglotVersion()
}

// Gets the price (in evm gas basis points) of wasm gas
func (con ArbWasm) WasmGasPrice(c ctx, evm mech) (uint64, error) {
	bips, err := c.State.Programs().WasmGasPrice()
	return bips.Uint64(), err
}

// Gets the wasm stack size limit
func (con ArbWasm) WasmMaxDepth(c ctx, evm mech) (uint32, error) {
	return c.State.Programs().WasmMaxDepth()
}

// Gets the wasm memory limit
func (con ArbWasm) WasmHeapBound(c ctx, evm mech) (uint32, error) {
	return c.State.Programs().WasmHeapBound()
}