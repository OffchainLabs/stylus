// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package arbtest

import (
	"context"
	"fmt"
	"os"
	"testing"
	"time"

	"bytes"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/common/hexutil"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/params"
	"github.com/offchainlabs/nitro/arbcompress"
	"github.com/offchainlabs/nitro/arbnode"
	"github.com/offchainlabs/nitro/solgen/go/mocksgen"
	"github.com/offchainlabs/nitro/solgen/go/precompilesgen"
	"github.com/offchainlabs/nitro/util/colors"
	"strings"
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

	auth := l2info.GetDefaultTransactOpts("Owner", ctx)
	arbWasm, err := precompilesgen.NewArbWasm(common.HexToAddress("0x71"), l2client)
	Require(t, err)

	file := "../arbitrator/stylus/tests/keccak/target/wasm32-unknown-unknown/release/keccak.wasm"
	wasmSource, err := os.ReadFile(file)
	Require(t, err)
	wasm, err := arbcompress.CompressWell(wasmSource)
	Require(t, err)

	stylusWasmPrefix := hexutil.MustDecode("0xEF0000")
	code := append(stylusWasmPrefix, wasm...)

	toKb := func(data []byte) float64 { return float64(len(data)) / 1024.0 }
	colors.PrintMint(fmt.Sprintf("WASM len %.2fK vs %.2fK", toKb(code), toKb(wasmSource)))

	ensure := func(tx *types.Transaction, err error) *types.Receipt {
		t.Helper()
		Require(t, err)
		receipt, err := EnsureTxSucceeded(ctx, l2client, tx)
		Require(t, err)
		return receipt
	}

	timed := func(message string, lambda func()) {
		t.Helper()
		now := time.Now()
		lambda()
		passed := time.Since(now)
		colors.PrintBlue("Time to ", message, ": ", passed.String())
	}

	programAddress := deployContract(t, ctx, auth, l2client, code)
	colors.PrintBlue("program deployed to ", programAddress.Hex())

	timed("compile", func() {
		ensure(arbWasm.CompileProgram(&auth, programAddress))
	})

	preimage := []byte("°º¤ø,¸,ø¤°º¤ø,¸,ø¤°º¤ø,¸ nyan nyan ~=[,,_,,]:3 nyan nyan")
	correct := crypto.Keccak256Hash(preimage)

	args := []byte{0x01} // keccak the preimage once
	args = append(args, preimage...)

	timed("execute", func() {
		colors.PrintMint("Sending non-mutating call to contract as a normal Ethereum tx")

		result := sendContractCall(t, ctx, programAddress, l2client, args)

		def := `[{"inputs":[{"name":"","type":"address"}, {"name":"", "type":"bytes"}],"name":"callProgram","outputs":[{"name":"status","type":"uint32"}, {"name": "result", "type":"bytes"}],"type":"function"}]`
		abi, err := abi.JSON(strings.NewReader(def))
		Require(t, err)
		callResults, err := abi.Unpack("callProgram", result)
		Require(t, err)
		status := callResults[0].(uint32)

		colors.PrintMint("Status = ", status)

		rawHash := callResults[1].([]byte)

		if len(rawHash) != 32 {
			Fail(t, "unexpected return result", result)
		}
		if !bytes.Equal(rawHash, correct[:]) {
			Fail(t, "computed hash mismatch", fmt.Sprintf("%#x", rawHash), correct)
		}
		colors.PrintGrey("keccak(x) = ", fmt.Sprintf("%#x", rawHash))
	})

	// do a mutating call for proving's sake
	_, tx, mock, err := mocksgen.DeployProgramTest(&auth, l2client)
	ensure(tx, err)
	ensure(mock.CallKeccak(&auth, programAddress, args))

	doUntil(t, 20*time.Millisecond, 50, func() bool {
		batchCount, err := node.InboxTracker.GetBatchCount()
		Require(t, err)
		meta, err := node.InboxTracker.GetBatchMetadata(batchCount - 1)
		Require(t, err)
		messageCount, err := node.ArbInterface.TransactionStreamer().GetMessageCount()
		Require(t, err)
		return meta.MessageCount == messageCount
	})
}
