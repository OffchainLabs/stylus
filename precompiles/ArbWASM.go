// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package precompiles

import (
	"errors"

	"github.com/ethereum/go-ethereum/log"
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
	// TODO: require some intrinsic amount of gas
	programs := c.State.Programs()

	// give all gas to the program
	getGas := func() uint64 { return c.gasLeft }
	gasLeft, status, output, err := programs.CallProgram(evm.StateDB, address, calldata, getGas)
	if err != nil {
		return 0, nil, err
	}

	if gasLeft > c.gasLeft {
		log.Error("program gas didn't decrease", "gas", c.gasLeft, "gasLeft", gasLeft)
		return 0, nil, errors.New("internal metering error")
	}
	return status, output, c.Burn(c.gasLeft - gasLeft)
}
