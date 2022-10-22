// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package precompiles

type ArbWASM struct {
	Address addr // 0xa0

	ProgramAdded        func(ctx, mech, bytes32) error
	ProgramAddedGasCost func(bytes32) (uint64, error)
}

func (con *ArbWASM) AddProgram(c ctx, evm mech, wasm []byte) (bytes32, error) {
	// TODO: pay for gas by some sizing formula
	wasm_hash := c.State.Programs().AddProgram(evm.StateDB, wasm)
	return wasm_hash, con.ProgramAdded(c, evm, wasm_hash)
}

func (con *ArbWASM) CompileProgram(c ctx, evm mech, wasm_hash bytes32) error {
	// TODO: pay for gas by some compilation pricing formula
	return c.State.Programs().CompileProgram(evm.StateDB, wasm_hash)
}

func (con *ArbWASM) CallProgram(c ctx, evm mech, wasm_hash bytes32, calldata []byte) (uint64, []byte, error) {
	// TODO: require some intrinsic amount of gas
	programs := c.State.Programs()

	// give all gas to the program
	burnt, status, output, err := programs.CallProgram(evm.StateDB, wasm_hash, calldata, c.gasLeft)
	if err != nil {
		return 0, nil, err
	}
	return status, output, c.Burn(burnt)
}
