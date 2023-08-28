// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package precompiles

type ArbWasm struct {
	Address addr // 0x71

	ProgramNotActivatedError func() error
	ProgramOutOfDateError    func(version uint16) error
	ProgramUpToDateError     func() error
}

// Compile a wasm program with the latest instrumentation
func (con ArbWasm) ActivateProgram(c ctx, evm mech, program addr) (uint16, error) {
	version, takeAllGas, err := c.State.Programs().ActivateProgram(evm, program, evm.ChainConfig().DebugMode())
	if takeAllGas {
		_ = c.BurnOut()
		return version, err
	}
	return version, err
}

// Gets the latest stylus version
func (con ArbWasm) StylusVersion(c ctx, _ mech) (uint16, error) {
	return c.State.Programs().StylusVersion()
}

// Gets the amount of ink 1 gas buys
func (con ArbWasm) InkPrice(c ctx, _ mech) (uint32, error) {
	ink, err := c.State.Programs().InkPrice()
	return ink.ToUint32(), err
}

// Gets the wasm stack size limit
func (con ArbWasm) MaxStackDepth(c ctx, _ mech) (uint32, error) {
	return c.State.Programs().MaxStackDepth()
}

// Gets the number of free wasm pages a tx gets
func (con ArbWasm) FreePages(c ctx, _ mech) (uint16, error) {
	return c.State.Programs().FreePages()
}

// Gets the base cost of each additional wasm page
func (con ArbWasm) PageGas(c ctx, _ mech) (uint16, error) {
	return c.State.Programs().PageGas()
}

// Gets the ramp that drives exponential memory costs
func (con ArbWasm) PageRamp(c ctx, _ mech) (uint64, error) {
	return c.State.Programs().PageRamp()
}

// Gets the maximum initial number of pages a wasm may allocate
func (con ArbWasm) PageLimit(c ctx, _ mech) (uint16, error) {
	return c.State.Programs().PageLimit()
}

// Gets the current program version
func (con ArbWasm) ProgramVersion(c ctx, evm mech, program addr) (uint16, error) {
	codehash, err := c.GetCodeHash(program)
	if err != nil {
		return 0, err
	}
	return c.State.Programs().ProgramVersion(codehash)
}

// Gets the added wasm call cost paid per half kb uncompressed wasm
func (con ArbWasm) CallScalar(c ctx, _ mech) (uint16, error) {
	return c.State.Programs().CallScalar()
}
