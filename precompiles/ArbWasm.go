// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package precompiles

type ArbWasm struct {
	Address addr // 0x71

	ProgramNotCompiledError func() error
	ProgramOutOfDateError   func(version uint32) error
	ProgramUpToDateError    func() error
}

// Compile a wasm program with the latest instrumentation
func (con ArbWasm) CompileProgram(c ctx, evm mech, program addr) (uint32, error) {
	// TODO: pay for gas by some compilation pricing formula
	version, takeAllGas, err := c.State.Programs().CompileProgram(evm, program, evm.ChainConfig().DebugMode())
	if takeAllGas {
		return version, c.BurnOut()
	}
	return version, err
}

// Gets the latest stylus version
func (con ArbWasm) StylusVersion(c ctx, _ mech) (uint32, error) {
	return c.State.Programs().StylusVersion()
}

// Gets the price (in evm gas basis points) of ink
func (con ArbWasm) InkPrice(c ctx, _ mech) (uint64, error) {
	bips, err := c.State.Programs().InkPrice()
	return bips.Uint64(), err
}

// Gets the wasm stack size limit
func (con ArbWasm) MaxStackDepth(c ctx, _ mech) (uint32, error) {
	return c.State.Programs().MaxStackDepth()
}

// Gets the cost of starting a stylus hostio call
func (con ArbWasm) HostioInk(c ctx, _ mech) (uint64, error) {
	return c.State.Programs().HostioInk()
}

// Gets the number of free wasm pages a tx gets
func (con ArbWasm) FreePages(c ctx, _ mech) (uint16, error) {
	return c.State.Programs().FreePages()
}

// Gets the base cost of each additional wasm page
func (con ArbWasm) PageGas(c ctx, _ mech) (uint32, error) {
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

// Gets the call overhead priced per half of a kb of compressed wasm
func (con ArbWasm) CallScalar(c ctx, _ mech) (uint16, error) {
	return c.State.Programs().CallScalar()
}

// Gets the current program version
func (con ArbWasm) ProgramVersion(c ctx, _ mech, program addr) (uint32, error) {
	return c.State.Programs().ProgramVersion(program)
}
