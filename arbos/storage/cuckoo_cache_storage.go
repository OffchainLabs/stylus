// Copyright 2021-2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package storage

import (
	"github.com/ethereum/go-ethereum/common"
	"github.com/offchainlabs/cuckoocache/onChainStorage"
)

type CuckooStorage struct {
	sto *Storage
}

type CuckooSlot struct {
	slot StorageSlot
}

func (c *CuckooSlot) Get() (common.Hash, error) {
	return c.slot.Get()
}

func (c *CuckooSlot) Set(value common.Hash) error {
	return c.slot.Set(value)
}

func (c *CuckooStorage) Get(location common.Hash) (common.Hash, error) {
	return c.sto.Get(location)
}

func (c *CuckooStorage) Set(location, value common.Hash) error {
	return c.sto.Set(location, value)
}

func (c *CuckooStorage) NewSlot(offset uint64) onChainStorage.OnChainStorageSlot {
	return &CuckooSlot{c.sto.NewSlot(offset)}
}

func (sto *Storage) ToCuckoo() onChainStorage.OnChainStorage {
	return &CuckooStorage{sto}
}
