// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

use arbitrum::{
    extractors::{trigger, Calldata, Context},
    Bytes20, Bytes32,
};

// Note: temporarily using printlns until
// returndata is implemented, run with
// cargo test -- --nocapture to view
fn print_success(msg: String) {
    println!("SUCCESS: {}", msg);
}

#[test]
fn empty_handler() {
    let _context = Context {
        calldata: Calldata(()),
    };

    fn empty_handler() {
        print_success("empty_handler".to_string())
    }

    trigger(&_context, empty_handler);
}

#[test]
fn extractors_one() {
    let ctx = Context {
        calldata: Calldata(Bytes32::from(1234u64)),
    };

    fn calldata_extractor(Calldata(qty): &Calldata<Bytes32>) {
        print_success(format!("One Extractor - Calldata; qty: {}", qty));
        // clone required here for Bytes32 type?
        assert_eq!(Bytes32::from(1234u64), qty.clone());
    }

    trigger(&ctx, calldata_extractor);

    let ctx = Context {
        calldata: Calldata(2345u64),
    };

    fn context_extractor(ctx: &Context<u64>) {
        print_success(format!("One Extractor - Context; ctx: {:?}", ctx));

        assert_eq!(2345u64, ctx.calldata.0);
    }

    trigger(&ctx, context_extractor);
}

#[test]
fn extractors_two() {
    let ctx = Context {
        calldata: Calldata(1234u64),
    };

    fn double_extractor(Calldata(qty): &Calldata<u64>, ctx: &Context<u64>) {
        print_success(format!(
            "Two Extractors - Calldata, Context; qty: {}, ctx: {:?}",
            qty, ctx
        ));

        assert_eq!(ctx.calldata.0.clone(), qty.clone());
        assert_eq!(qty.clone(), 1234u64);
    }

    trigger(&ctx, double_extractor);
}

#[test]
fn calldata_extractor_multiple_params() {
    type TransferCalldata = (Bytes20, Bytes32);

    let ctx = Context {
        calldata: Calldata((Bytes20::from(111u32), Bytes32::from(999u32))),
    };

    fn calldata_two_params(Calldata((to, amount)): &Calldata<TransferCalldata>) {
        print_success(format!(
            "Calldata Extractor - Two Params; to: {}, amount: {}",
            to, amount
        ));

        assert_eq!(to.clone(), Bytes20::from(111u32));
        assert_eq!(amount.clone(), Bytes32::from(999u32));
    }

    trigger(&ctx, calldata_two_params);

    struct TransferFromCalldata {
        from: Bytes20,
        to: Bytes20,
        amount: Bytes32,
    }

    let ctx = Context {
        calldata: Calldata(TransferFromCalldata {
            from: Bytes20::from(111u32),
            to: Bytes20::from(222u32),
            amount: Bytes32::from(999u64),
        }),
    };

    fn calldata_three_params(
        Calldata(TransferFromCalldata { to, from, amount }): &Calldata<TransferFromCalldata>,
    ) {
        print_success(format!(
            "Calldata Extractor - Three Params; to: {}, from: {}, amount: {}",
            to, from, amount
        ));

        assert_eq!(from.clone(), Bytes20::from(111u32));
        assert_eq!(to.clone(), Bytes20::from(222u32));
        assert_eq!(amount.clone(), Bytes32::from(999u32));
    }

    trigger(&ctx, calldata_three_params);
}
