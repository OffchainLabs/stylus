// Copyright 2021-2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package arbnode

import (
	"bytes"
	"context"
	"encoding/binary"
	"fmt"
	"sync"
	"time"

	"github.com/ethereum/go-ethereum/ethdb"
	"github.com/ethereum/go-ethereum/log"

	"github.com/offchainlabs/nitro/arbutil"
	"github.com/offchainlabs/nitro/util/stopwaiter"
	"github.com/offchainlabs/nitro/validator"

	flag "github.com/spf13/pflag"
)

type MessagePruner struct {
	stopwaiter.StopWaiter
	transactionStreamer *TransactionStreamer
	inboxTracker        *InboxTracker
	config              MessagePrunerConfigFetcher
	pruningLock         sync.Mutex
	lastPruneDone       time.Time
}

type MessagePrunerConfig struct {
	Enable               bool          `koanf:"enable"`
	MessagePruneInterval time.Duration `koanf:"prune-interval" reload:"hot"`
	MinBatchesLeft       uint64        `koanf:"min-batches-left" reload:"hot"`
}

type MessagePrunerConfigFetcher func() *MessagePrunerConfig

var DefaultMessagePrunerConfig = MessagePrunerConfig{
	Enable:               true,
	MessagePruneInterval: time.Minute,
	MinBatchesLeft:       2,
}

func MessagePrunerConfigAddOptions(prefix string, f *flag.FlagSet) {
	f.Bool(prefix+".enable", DefaultMessagePrunerConfig.Enable, "enable message pruning")
	f.Duration(prefix+".prune-interval", DefaultMessagePrunerConfig.MessagePruneInterval, "interval for running message pruner")
	f.Uint64(prefix+".min-batches-left", DefaultMessagePrunerConfig.MinBatchesLeft, "min number of batches not pruned")
}

func NewMessagePruner(transactionStreamer *TransactionStreamer, inboxTracker *InboxTracker, config MessagePrunerConfigFetcher) *MessagePruner {
	return &MessagePruner{
		transactionStreamer: transactionStreamer,
		inboxTracker:        inboxTracker,
		config:              config,
	}
}

func (m *MessagePruner) Start(ctxIn context.Context) {
	m.StopWaiter.Start(ctxIn, m)
}

func (m *MessagePruner) UpdateLatestStaked(count arbutil.MessageIndex, globalState validator.GoGlobalState) {
	locked := m.pruningLock.TryLock()
	if !locked {
		return
	}

	if m.lastPruneDone.Add(m.config().MessagePruneInterval).After(time.Now()) {
		m.pruningLock.Unlock()
		return
	}
	err := m.LaunchThreadSafe(func(ctx context.Context) {
		defer m.pruningLock.Unlock()
		err := m.prune(ctx, count, globalState)
		if err != nil && ctx.Err() == nil {
			log.Error("error while pruning", "err", err)
		}
	})
	if err != nil {
		log.Info("failed launching prune thread", "err", err)
		m.pruningLock.Unlock()
	}
}

func (m *MessagePruner) prune(ctx context.Context, count arbutil.MessageIndex, globalState validator.GoGlobalState) error {
	trimBatchCount := globalState.Batch
	minBatchesLeft := m.config().MinBatchesLeft
	batchCount, err := m.inboxTracker.GetBatchCount()
	if err != nil {
		return err
	}
	if batchCount < trimBatchCount+minBatchesLeft {
		if batchCount < minBatchesLeft {
			return nil
		}
		trimBatchCount = batchCount - minBatchesLeft
	}
	if trimBatchCount < 1 {
		return nil
	}
	endBatchMetadata, err := m.inboxTracker.GetBatchMetadata(trimBatchCount - 1)
	if err != nil {
		return err
	}
	msgCount := endBatchMetadata.MessageCount
	delayedCount := endBatchMetadata.DelayedMessageCount

	return deleteOldMessageFromDB(ctx, msgCount, delayedCount, m.inboxTracker.db, m.transactionStreamer.db)
}

func deleteOldMessageFromDB(ctx context.Context, messageCount arbutil.MessageIndex, delayedMessageCount uint64, inboxTrackerDb ethdb.Database, transactionStreamerDb ethdb.Database) error {
	prunedKeysRange, err := deleteFromLastPrunedUptoEndKey(ctx, transactionStreamerDb, messagePrefix, uint64(messageCount))
	if err != nil {
		return fmt.Errorf("error deleting last batch messages: %w", err)
	}
	if len(prunedKeysRange) > 0 {
		log.Info("Pruned last batch messages:", "first pruned key", prunedKeysRange[0], "last pruned key", prunedKeysRange[len(prunedKeysRange)-1])
	}

	prunedKeysRange, err = deleteFromLastPrunedUptoEndKey(ctx, inboxTrackerDb, rlpDelayedMessagePrefix, delayedMessageCount)
	if err != nil {
		return fmt.Errorf("error deleting last batch delayed messages: %w", err)
	}
	if len(prunedKeysRange) > 0 {
		log.Info("Pruned last batch delayed messages:", "first pruned key", prunedKeysRange[0], "last pruned key", prunedKeysRange[len(prunedKeysRange)-1])
	}
	return nil
}

func deleteFromLastPrunedUptoEndKey(ctx context.Context, db ethdb.Database, prefix []byte, endMinKey uint64) ([]uint64, error) {
	startIter := db.NewIterator(prefix, uint64ToKey(1))
	if !startIter.Next() {
		return nil, nil
	}
	startMinKey := binary.BigEndian.Uint64(bytes.TrimPrefix(startIter.Key(), prefix))
	startIter.Release()
	if endMinKey > startMinKey {
		return deleteFromRange(ctx, db, prefix, startMinKey, endMinKey-1)
	}
	return nil, nil
}
