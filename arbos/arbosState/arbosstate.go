// Copyright 2021-2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package arbosState

import (
	"errors"
	"fmt"
	"math/big"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/common/math"
	"github.com/ethereum/go-ethereum/core/rawdb"
	"github.com/ethereum/go-ethereum/core/state"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/log"
	"github.com/ethereum/go-ethereum/params"

	"github.com/offchainlabs/nitro/arbos/addressSet"
	"github.com/offchainlabs/nitro/arbos/addressTable"
	"github.com/offchainlabs/nitro/arbos/arbostypes"
	"github.com/offchainlabs/nitro/arbos/blockhash"
	"github.com/offchainlabs/nitro/arbos/burn"
	"github.com/offchainlabs/nitro/arbos/l1pricing"
	"github.com/offchainlabs/nitro/arbos/l2pricing"
	"github.com/offchainlabs/nitro/arbos/merkleAccumulator"
	"github.com/offchainlabs/nitro/arbos/programs"
	"github.com/offchainlabs/nitro/arbos/retryables"
	"github.com/offchainlabs/nitro/arbos/storage"
	"github.com/offchainlabs/nitro/arbos/util"
)

// ArbosState contains ArbOS-related state. It is backed by ArbOS's storage in the persistent stateDB.
// Modifications to the ArbosState are written through to the underlying StateDB so that the StateDB always
// has the definitive state, stored persistently. (Note that some tests use memory-backed StateDB's that aren't
// persisted beyond the end of the test.)

type ArbosState struct {
	arbosVersion      uint64                      // version of the ArbOS storage format and semantics
	upgradeVersion    storage.StorageBackedUint64 // version we're planning to upgrade to, or 0 if not planning to upgrade
	upgradeTimestamp  storage.StorageBackedUint64 // when to do the planned upgrade
	networkFeeAccount storage.StorageBackedAddress
	l1PricingState    *l1pricing.L1PricingState
	l2PricingState    *l2pricing.L2PricingState
	retryableState    *retryables.RetryableState
	addressTable      *addressTable.AddressTable
	chainOwners       *addressSet.AddressSet
	sendMerkle        *merkleAccumulator.MerkleAccumulator
	programs          *programs.Programs
	blockhashes       *blockhash.Blockhashes
	chainId           storage.StorageBackedBigInt
	chainConfig       storage.StorageBackedBytes
	genesisBlockNum   storage.StorageBackedUint64
	infraFeeAccount   storage.StorageBackedAddress
	backingStorage    *storage.Storage
	Burner            burn.Burner
}

var ErrUninitializedArbOS = errors.New("ArbOS uninitialized")
var ErrAlreadyInitialized = errors.New("ArbOS is already initialized")

func OpenArbosState(stateDB vm.StateDB, burner burn.Burner) (*ArbosState, error) {
	backingStorage := storage.NewGeth(stateDB, burner)
	arbosVersion, err := backingStorage.GetUint64ByUint64(uint64(versionOffset))
	if err != nil {
		return nil, err
	}
	if arbosVersion == 0 {
		return nil, ErrUninitializedArbOS
	}
	return &ArbosState{
		arbosVersion,
		backingStorage.OpenStorageBackedUint64(uint64(upgradeVersionOffset)),
		backingStorage.OpenStorageBackedUint64(uint64(upgradeTimestampOffset)),
		backingStorage.OpenStorageBackedAddress(uint64(networkFeeAccountOffset)),
		l1pricing.OpenL1PricingState(backingStorage.OpenSubStorage(l1PricingSubspace)),
		l2pricing.OpenL2PricingState(backingStorage.OpenSubStorage(l2PricingSubspace)),
		retryables.OpenRetryableState(backingStorage.OpenSubStorage(retryablesSubspace), stateDB),
		addressTable.Open(backingStorage.OpenSubStorage(addressTableSubspace)),
		addressSet.OpenAddressSet(backingStorage.OpenSubStorage(chainOwnerSubspace)),
		merkleAccumulator.OpenMerkleAccumulator(backingStorage.OpenSubStorage(sendMerkleSubspace)),
		programs.Open(backingStorage.OpenSubStorage(programsSubspace)),
		blockhash.OpenBlockhashes(backingStorage.OpenSubStorage(blockhashesSubspace)),
		backingStorage.OpenStorageBackedBigInt(uint64(chainIdOffset)),
		backingStorage.OpenStorageBackedBytes(chainConfigSubspace),
		backingStorage.OpenStorageBackedUint64(uint64(genesisBlockNumOffset)),
		backingStorage.OpenStorageBackedAddress(uint64(infraFeeAccountOffset)),
		backingStorage,
		burner,
	}, nil
}

func OpenSystemArbosState(stateDB vm.StateDB, tracingInfo *util.TracingInfo, readOnly bool) (*ArbosState, error) {
	burner := burn.NewSystemBurner(tracingInfo, readOnly)
	newState, err := OpenArbosState(stateDB, burner)
	burner.Restrict(err)
	return newState, err
}

func OpenSystemArbosStateOrPanic(stateDB vm.StateDB, tracingInfo *util.TracingInfo, readOnly bool) *ArbosState {
	newState, err := OpenSystemArbosState(stateDB, tracingInfo, readOnly)
	if err != nil {
		panic(err)
	}
	return newState
}

// NewArbosMemoryBackedArbOSState creates and initializes a memory-backed ArbOS state (for testing only)
func NewArbosMemoryBackedArbOSState(
	arbosVersionPrecompileAddresses map[uint64][]common.Address,
) (*ArbosState, *state.StateDB) {
	raw := rawdb.NewMemoryDatabase()
	db := state.NewDatabase(raw)
	statedb, err := state.New(common.Hash{}, db, nil)
	if err != nil {
		log.Crit("failed to init empty statedb", "error", err)
	}
	burner := burn.NewSystemBurner(nil, false)
	chainConfig := params.ArbitrumDevTestChainConfig()
	newState, err := InitializeArbosState(statedb, burner, chainConfig, arbostypes.TestInitMessage, arbosVersionPrecompileAddresses)
	if err != nil {
		log.Crit("failed to open the ArbOS state", "error", err)
	}
	return newState, statedb
}

// ArbOSVersion returns the ArbOS version
func ArbOSVersion(stateDB vm.StateDB) uint64 {
	backingStorage := storage.NewGeth(stateDB, burn.NewSystemBurner(nil, false))
	arbosVersion, err := backingStorage.GetUint64ByUint64(uint64(versionOffset))
	if err != nil {
		log.Crit("failed to get the ArbOS version", "error", err)
	}
	return arbosVersion
}

type Offset uint64

const (
	versionOffset Offset = iota
	upgradeVersionOffset
	upgradeTimestampOffset
	networkFeeAccountOffset
	chainIdOffset
	genesisBlockNumOffset
	infraFeeAccountOffset
)

type SubspaceID []byte

var (
	l1PricingSubspace    SubspaceID = []byte{0}
	l2PricingSubspace    SubspaceID = []byte{1}
	retryablesSubspace   SubspaceID = []byte{2}
	addressTableSubspace SubspaceID = []byte{3}
	chainOwnerSubspace   SubspaceID = []byte{4}
	sendMerkleSubspace   SubspaceID = []byte{5}
	blockhashesSubspace  SubspaceID = []byte{6}
	chainConfigSubspace  SubspaceID = []byte{7}
	programsSubspace     SubspaceID = []byte{8}
)

func InitializeArbosState(
	stateDB vm.StateDB,
	burner burn.Burner,
	chainConfig *params.ChainConfig,
	initMessage *arbostypes.ParsedInitMessage,
	arbosVersionPrecompileAddresses map[uint64][]common.Address,
) (*ArbosState, error) {
	sto := storage.NewGeth(stateDB, burner)
	arbosVersion, err := sto.GetUint64ByUint64(uint64(versionOffset))
	if err != nil {
		return nil, err
	}
	if arbosVersion != 0 {
		return nil, ErrAlreadyInitialized
	}

	desiredArbosVersion := chainConfig.ArbitrumChainParams.InitialArbOSVersion
	if desiredArbosVersion == 0 {
		return nil, errors.New("cannot initialize to ArbOS version 0")
	}

	// may be the zero address
	initialChainOwner := chainConfig.ArbitrumChainParams.InitialChainOwner

	_ = sto.SetUint64ByUint64(uint64(versionOffset), 1) // initialize to version 1; upgrade at end of this func if needed
	_ = sto.SetUint64ByUint64(uint64(upgradeVersionOffset), 0)
	_ = sto.SetUint64ByUint64(uint64(upgradeTimestampOffset), 0)
	if desiredArbosVersion >= 2 {
		_ = sto.SetByUint64(uint64(networkFeeAccountOffset), util.AddressToHash(initialChainOwner))
	} else {
		_ = sto.SetByUint64(uint64(networkFeeAccountOffset), common.Hash{}) // the 0 address until an owner sets it
	}
	_ = sto.SetByUint64(uint64(chainIdOffset), common.BigToHash(chainConfig.ChainID))
	chainConfigStorage := sto.OpenStorageBackedBytes(chainConfigSubspace)
	_ = chainConfigStorage.Set(initMessage.SerializedChainConfig)
	_ = sto.SetUint64ByUint64(uint64(genesisBlockNumOffset), chainConfig.ArbitrumChainParams.GenesisBlockNum)

	initialRewardsRecipient := l1pricing.BatchPosterAddress
	if desiredArbosVersion >= 2 {
		initialRewardsRecipient = initialChainOwner
	}
	_ = l1pricing.InitializeL1PricingState(sto.OpenSubStorage(l1PricingSubspace), initialRewardsRecipient, initMessage.InitialL1BaseFee)
	_ = l2pricing.InitializeL2PricingState(sto.OpenSubStorage(l2PricingSubspace))
	_ = retryables.InitializeRetryableState(sto.OpenSubStorage(retryablesSubspace))
	addressTable.Initialize(sto.OpenSubStorage(addressTableSubspace))
	merkleAccumulator.InitializeMerkleAccumulator(sto.OpenSubStorage(sendMerkleSubspace))
	blockhash.InitializeBlockhashes(sto.OpenSubStorage(blockhashesSubspace))

	ownersStorage := sto.OpenSubStorage(chainOwnerSubspace)
	_ = addressSet.Initialize(ownersStorage)
	_ = addressSet.OpenAddressSet(ownersStorage).Add(initialChainOwner)

	aState, err := OpenArbosState(stateDB, burner)
	if err != nil {
		return nil, err
	}
	if desiredArbosVersion > 1 {
		err = aState.UpgradeArbosVersion(desiredArbosVersion, true, stateDB, chainConfig, arbosVersionPrecompileAddresses)
		if err != nil {
			return nil, err
		}
	}
	return aState, nil
}

func (state *ArbosState) UpgradeArbosVersionIfNecessary(
	currentTimestamp uint64,
	stateDB vm.StateDB,
	chainConfig *params.ChainConfig,
	arbosVersionPrecompileAddresses map[uint64][]common.Address,
) error {
	upgradeTo, err := state.upgradeVersion.Get()
	state.Restrict(err)
	flagday, _ := state.upgradeTimestamp.Get()
	if state.arbosVersion < upgradeTo && currentTimestamp >= flagday {
		return state.UpgradeArbosVersion(upgradeTo, false, stateDB, chainConfig, arbosVersionPrecompileAddresses)
	}
	return nil
}

var ErrFatalNodeOutOfDate = errors.New("please upgrade to the latest version of the node software")

func (state *ArbosState) UpgradeArbosVersion(
	upgradeTo uint64,
	firstTime bool,
	stateDB vm.StateDB,
	chainConfig *params.ChainConfig,
	arbosVersionPrecompileAddresses map[uint64][]common.Address,
) error {
	for state.arbosVersion < upgradeTo {
		ensure := func(err error) {
			if err != nil {
				message := fmt.Sprintf(
					"Failed to upgrade ArbOS version %v to version %v: %v",
					state.arbosVersion, state.arbosVersion+1, err,
				)
				panic(message)
			}
		}

		// Solidity requires call targets have code, but precompiles don't.
		// To work around this, we give precompiles fake code.
		for _, genesisPrecompile := range arbosVersionPrecompileAddresses[state.arbosVersion] {
			stateDB.SetCode(genesisPrecompile, []byte{byte(vm.INVALID)})
		}

		switch state.arbosVersion {
		case 1:
			ensure(state.l1PricingState.SetLastSurplus(common.Big0, 1))
		case 2:
			ensure(state.l1PricingState.SetPerBatchGasCost(0))
			ensure(state.l1PricingState.SetAmortizedCostCapBips(math.MaxUint64))
		case 3:
			// no state changes needed
		case 4:
			// no state changes needed
		case 5:
			// no state changes needed
		case 6:
			// no state changes needed
		case 7:
			// no state changes needed
		case 8:
			// no state changes needed
		case 9:
			ensure(state.l1PricingState.SetL1FeesAvailable(stateDB.GetBalance(
				l1pricing.L1PricerFundsPoolAddress,
			)))
		case 10:
			if !chainConfig.DebugMode() {
				// This upgrade isn't finalized so we only want to support it for testing
				return fmt.Errorf(
					"the chain is upgrading to unsupported ArbOS version %v, %w",
					state.arbosVersion+1,
					ErrFatalNodeOutOfDate,
				)
			}
			// Update the PerBatchGasCost to a more accurate value compared to the old v6 default.
			ensure(state.l1PricingState.SetPerBatchGasCost(l1pricing.InitialPerBatchGasCostV12))

			// We had mistakenly initialized AmortizedCostCapBips to math.MaxUint64 in older versions,
			// but the correct value to disable the amortization cap is 0.
			oldAmortizationCap, err := state.l1PricingState.AmortizedCostCapBips()
			ensure(err)
			if oldAmortizationCap == math.MaxUint64 {
				ensure(state.l1PricingState.SetAmortizedCostCapBips(0))
			}

			// Clear chainOwners list to allow rectification of the mapping.
			if !firstTime {
				ensure(state.chainOwners.ClearList())
			}
		default:
			return fmt.Errorf(
				"the chain is upgrading to unsupported ArbOS version %v, %w",
				state.arbosVersion+1,
				ErrFatalNodeOutOfDate,
			)
		}
		state.arbosVersion++
	}

	if firstTime && upgradeTo >= 6 {
		if upgradeTo < 11 {
			state.Restrict(state.l1PricingState.SetPerBatchGasCost(l1pricing.InitialPerBatchGasCostV6))
		}
		if chainConfig.ArbitrumStylusEnabled(upgradeTo) {
			programs.Initialize(state.backingStorage.OpenSubStorage(programsSubspace))
		}
		state.Restrict(state.l1PricingState.SetEquilibrationUnits(l1pricing.InitialEquilibrationUnitsV6))
		state.Restrict(state.l2PricingState.SetSpeedLimitPerSecond(l2pricing.InitialSpeedLimitPerSecondV6))
		state.Restrict(state.l2PricingState.SetMaxPerBlockGasLimit(l2pricing.InitialPerBlockGasLimitV6))
	}

	state.Restrict(state.backingStorage.SetUint64ByUint64(uint64(versionOffset), state.arbosVersion))

	return nil
}

func (state *ArbosState) ScheduleArbOSUpgrade(newVersion uint64, timestamp uint64) error {
	err := state.upgradeVersion.Set(newVersion)
	if err != nil {
		return err
	}
	return state.upgradeTimestamp.Set(timestamp)
}

func (state *ArbosState) GetScheduledUpgrade() (uint64, uint64, error) {
	version, err := state.upgradeVersion.Get()
	if err != nil {
		return 0, 0, err
	}
	timestamp, err := state.upgradeTimestamp.Get()
	if err != nil {
		return 0, 0, err
	}
	return version, timestamp, nil
}

func (state *ArbosState) BackingStorage() *storage.Storage {
	return state.backingStorage
}

func (state *ArbosState) Restrict(err error) {
	state.Burner.Restrict(err)
}

func (state *ArbosState) ArbOSVersion() uint64 {
	return state.arbosVersion
}

func (state *ArbosState) SetFormatVersion(val uint64) {
	state.arbosVersion = val
	state.Restrict(state.backingStorage.SetUint64ByUint64(uint64(versionOffset), val))
}

func (state *ArbosState) RetryableState() *retryables.RetryableState {
	return state.retryableState
}

func (state *ArbosState) L1PricingState() *l1pricing.L1PricingState {
	return state.l1PricingState
}

func (state *ArbosState) L2PricingState() *l2pricing.L2PricingState {
	return state.l2PricingState
}

func (state *ArbosState) AddressTable() *addressTable.AddressTable {
	return state.addressTable
}

func (state *ArbosState) ChainOwners() *addressSet.AddressSet {
	return state.chainOwners
}

func (state *ArbosState) SendMerkleAccumulator() *merkleAccumulator.MerkleAccumulator {
	if state.sendMerkle == nil {
		state.sendMerkle = merkleAccumulator.OpenMerkleAccumulator(state.backingStorage.OpenSubStorage(sendMerkleSubspace))
	}
	return state.sendMerkle
}

func (state *ArbosState) Programs() *programs.Programs {
	return state.programs
}

func (state *ArbosState) Blockhashes() *blockhash.Blockhashes {
	return state.blockhashes
}

func (state *ArbosState) NetworkFeeAccount() (common.Address, error) {
	return state.networkFeeAccount.Get()
}

func (state *ArbosState) SetNetworkFeeAccount(account common.Address) error {
	return state.networkFeeAccount.Set(account)
}

func (state *ArbosState) InfraFeeAccount() (common.Address, error) {
	return state.infraFeeAccount.Get()
}

func (state *ArbosState) SetInfraFeeAccount(account common.Address) error {
	return state.infraFeeAccount.Set(account)
}

func (state *ArbosState) Keccak(data ...[]byte) ([]byte, error) {
	return state.backingStorage.Keccak(data...)
}

func (state *ArbosState) KeccakHash(data ...[]byte) (common.Hash, error) {
	return state.backingStorage.KeccakHash(data...)
}

func (state *ArbosState) ChainId() (*big.Int, error) {
	return state.chainId.Get()
}

func (state *ArbosState) ChainConfig() ([]byte, error) {
	return state.chainConfig.Get()
}

func (state *ArbosState) SetChainConfig(serializedChainConfig []byte) error {
	return state.chainConfig.Set(serializedChainConfig)
}

func (state *ArbosState) GenesisBlockNum() (uint64, error) {
	return state.genesisBlockNum.Get()
}
