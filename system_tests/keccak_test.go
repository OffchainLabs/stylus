// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package arbtest

import (
	"context"
	"math/big"
	"testing"
	"time"

	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/offchainlabs/nitro/solgen/go/ospgen"
	"github.com/offchainlabs/nitro/util/colors"
)

func TestKeccakEVM(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	l2info, node, client := CreateTestL2(t, ctx)
	defer node.StopAndWait()

	auth := l2info.GetDefaultTransactOpts("Owner", ctx)
	_, tx, helper, err := ospgen.DeployHashProofHelper(&auth, client)
	Require(t, err)
	_, err = EnsureTxSucceeded(ctx, client, tx)
	Require(t, err)

	preimage := []byte("°º¤ø,¸¸,ø¤º°`°º¤ø,¸,ø¤°º¤ø,¸¸,ø¤º°`°º¤ø,¸ nyan nyan ~=[,,_,,]:3 nyan nyan")
	correct := crypto.Keccak256Hash(preimage)

	now := time.Now()
	hash, err := helper.SoftHash(&bind.CallOpts{}, preimage, 0, big.NewInt(1))
	Require(t, err)
	if hash != correct {
		Fail(t, "computed hash mismatch", hash, correct)
	}

	passed := time.Since(now)
	colors.PrintMint("Time to execute ", passed.String())
}
