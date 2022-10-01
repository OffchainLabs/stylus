// Copyright 2022, Offchain Labs, Inc.
// For license information, see https://github.com/nitro/blob/master/LICENSE

#include "arbitrum.h"
#include "monocypher.h"
#include <stdlib.h>
#include <string.h>

ArbResult user_main(const uint8_t * args, size_t args_len) {
    uint8_t sk[32] = {0};
    uint8_t pk[32] = {0};
    crypto_sign_public_key(pk, sk);

    uint8_t * signature = malloc(64 * sizeof(*signature));
    const char * message = "✲´*。.❄¨¯`* ✲。(╯^□^)╯ <(yay, it's snowing!) ✲。❄。*。¨¯`*✲";
    const size_t length = strlen(message);

    crypto_sign(signature, sk, pk, (const uint8_t *) message, length);

    return (ArbResult) {
        .status = Success,
        .output = signature,
        .output_len = 64,
    };
}

ARBITRUM_MAIN(user_main);
