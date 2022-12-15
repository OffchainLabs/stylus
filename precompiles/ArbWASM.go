// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package precompiles

import (
	"errors"
)

type ArbWASM struct {
	Address addr // 0xa0

	ProgramAdded        func(ctx, mech, bytes32) error
	ProgramAddedGasCost func(bytes32) (uint64, error)
}

func (con *ArbWASM) CompileProgram(c ctx, evm mech, address addr) error {
	// TODO: pay for gas by some compilation pricing formula
	return c.State.Programs().CompileProgram(evm.StateDB, address)
}

func (con *ArbWASM) CallProgram(c ctx, evm mech, address addr, calldata []byte) (uint32, []byte, error) {
	return 0, nil, errors.New("deprecated")
}
