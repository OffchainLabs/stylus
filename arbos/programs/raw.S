// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

//go:build js
// +build js

#include "textflag.h"

TEXT ·polyCompile(SB), NOSPLIT, $0
  CallImport
  RET

TEXT ·polyExecute(SB), NOSPLIT, $0
  CallImport
  RET
