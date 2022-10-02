// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE
//
// Compiler flags
//     wasi-sdk/bin/clang *.c -target wasm32-wasi --sysroot wasi-sdk/share/wasi-sysroot -mexec-model=reactor
//     -Oz -flto -Wl,--no-entry -Wl,--lto-O3 -s -Wl,-s -Wl,-z,stack-size=$[1024 * 1024] -o eddsa.wasm
//

#include "arbitrum.h"
#include "monocypher.h"
#include "monocypher-ed25519.h"
#include <stdlib.h>

ArbResult user_main(const uint8_t * args, size_t args_len) {
    const uint8_t * signature = args;
    const uint8_t * pk = args + 64;
    const uint8_t * message = args + 96;
    const size_t length = args_len - 96;

    size_t valid = crypto_ed25519_check(signature, pk, message, length);

    return (ArbResult) {
        .status = valid,
        .output = NULL,
        .output_len = 0,
    };
}

ARBITRUM_MAIN(user_main);
