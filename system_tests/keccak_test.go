// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package arbtest

import (
	"context"
	"math/big"
	"testing"
	"time"

	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/offchainlabs/nitro/solgen/go/ospgen"
	"github.com/offchainlabs/nitro/util/colors"
)

func TestKeccak(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	l2info, _, client, l2stack := CreateTestL2(t, ctx)
	defer requireClose(t, l2stack)

	auth := l2info.GetDefaultTransactOpts("Owner", ctx)
	addr, tx, helper, err := ospgen.DeployHashProofHelper(&auth, client)
	Require(t, err)
	_, err = EnsureTxSucceeded(ctx, client, tx)
	Require(t, err)

	colors.PrintBlue(addr)

	now := time.Now()
	input := []byte("°º¤ø,¸¸,ø¤º°`°º¤ø,¸,ø¤°º¤ø,¸¸,ø¤º°`°º¤ø,¸ nyan nyan ~=[,,_,,]:3 nyan nyan")
	_, err = helper.SoftHash(&bind.CallOpts{}, input, 0, big.NewInt(1))
	Require(t, err)

	passed := time.Since(now)
	println(passed.String())
}
