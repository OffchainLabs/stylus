// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package arbtest

import (
	"context"
	"os"
	"testing"
	"time"

	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/params"
	"github.com/offchainlabs/nitro/arbcompress"
	"github.com/offchainlabs/nitro/arbnode"
	"github.com/offchainlabs/nitro/solgen/go/mocksgen"
	"github.com/offchainlabs/nitro/solgen/go/precompilesgen"
	"github.com/offchainlabs/nitro/util/arbmath"
	"github.com/offchainlabs/nitro/util/colors"
)

func TestKeccakProgram(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	chainConfig := params.ArbitrumDevTestChainConfig()
	l2config := arbnode.ConfigDefaultL1Test()
	l2config.BlockValidator.ArbitratorValidator = true
	l2config.BlockValidator.JitValidator = true
	l2config.BatchPoster.Enable = true
	l2config.L1Reader.Enable = true

	l2info, node, l2client, _, _, _, l1stack := createTestNodeOnL1WithConfig(t, ctx, true, l2config, chainConfig, nil)
	defer requireClose(t, l1stack)
	defer node.StopAndWait()

	if node.StatelessBlockValidator == nil {
		Fail(t, "no stateless block validator")
	}

	auth := l2info.GetDefaultTransactOpts("Owner", ctx)
	arbWASM, err := precompilesgen.NewArbWASM(common.HexToAddress("0xa0"), l2client)
	Require(t, err)

	file := "../arbitrator/polyglot/programs/sha3/target/wasm32-unknown-unknown/release/sha3.wasm"
	wasm, err := os.ReadFile(file)
	Require(t, err)
	wasm, err = arbcompress.CompressWell(wasm)
	Require(t, err)

	colors.PrintMint("WASM len ", len(wasm))

	ensure := func(tx *types.Transaction, err error) *types.Receipt {
		t.Helper()
		Require(t, err)
		receipt, err := EnsureTxSucceeded(ctx, l2client, tx)
		Require(t, err)
		return receipt
	}

	programAddress := deployContract(t, ctx, auth, l2client, wasm)
	colors.PrintBlue("program deployed to ", programAddress.Hex())

	ensure(arbWASM.CompileProgram(&auth, programAddress))

	preimage := []byte("°º¤ø,¸¸,ø¤º°`°º¤ø,¸,ø¤°º¤ø,¸¸,ø¤º°`°º¤ø,¸ nyan nyan ~=[,,_,,]:3 nyan nyan")
	correct := crypto.Keccak256Hash(preimage)

	now := time.Now()
	result, err := arbWASM.CallProgram(&bind.CallOpts{}, programAddress, preimage)
	Require(t, err)

	if result.Status != 0 || len(result.Result) != 32 {
		Fail(t, "unexpected return result: Status", result.Status, "Result:", result.Result)
	}

	hash := common.BytesToHash(result.Result)
	if hash != correct {
		Fail(t, "computed hash mismatch", hash, correct)
	}

	passed := time.Since(now)
	colors.PrintMint("Time to execute: ", passed.String())

	// do a mutating call for proving's sake
	_, tx, mock, err := mocksgen.DeployProgramTest(&auth, l2client)
	ensure(tx, err)
	ensure(mock.CallKeccak(&auth, programAddress, preimage))

	doUntil(t, 10*time.Millisecond, 10, func() bool {
		batchCount, err := node.InboxTracker.GetBatchCount()
		Require(t, err)
		meta, err := node.InboxTracker.GetBatchMetadata(batchCount - 1)
		Require(t, err)
		messageCount, err := node.ArbInterface.TransactionStreamer().GetMessageCount()
		Require(t, err)
		return meta.MessageCount == messageCount
	})

	blockHeight, err := l2client.BlockNumber(ctx)
	Require(t, err)

	success := true
	for block := uint64(1); block <= blockHeight; block++ {
		header, err := l2client.HeaderByNumber(ctx, arbmath.UintToBig(block))
		Require(t, err)

		correct, err := node.StatelessBlockValidator.ValidateBlock(ctx, header, true, common.Hash{})
		Require(t, err, block)
		if correct {
			colors.PrintMint("yay!! we validated block ", block)
		} else {
			colors.PrintRed("failed to validate block ", block)
		}
		success = success && correct
	}
	if !success {
		Fail(t)
	}
}
