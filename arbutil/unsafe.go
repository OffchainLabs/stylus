// Copyright 2021-2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

package arbutil

func SliceToPointer[T any](slice []T) *T {
	if len(slice) == 0 {
		return nil
	}
	return &slice[0]
}
