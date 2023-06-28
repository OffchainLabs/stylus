// Copyright 2022-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

// Context will contain all "global" values like
// Tx, Msg, Block, etc. For now, just Calldata is shown
#[derive(Debug)]
pub struct Context<CD> {
    pub calldata: Calldata<CD>,
}

// Here the generic CD refers to the user-defined
// struct or tuple for the handler params
#[derive(Debug)]
pub struct Calldata<CD>(pub CD);

// This trait must be implemented to define an extractor
// Given a reference to the context, it returns a reference
// to the defined substate of the context.
pub trait FromContext<CD> {
    fn from_context(ctx: &Context<CD>) -> &Self;
}

impl<CD> FromContext<CD> for Calldata<CD> {
    fn from_context(ctx: &Context<CD>) -> &Self {
        &ctx.calldata
    }
}

// If a dev wants to work with the context object directly
// They can use the Context<CD> extractor
impl<CD> FromContext<CD> for Context<CD> {
    fn from_context(ctx: &Context<CD>) -> &Self {
        &ctx
    }
}

// The Handler triat requires that a call function be defined
// for a given type and context
pub trait Handler<T, CD> {
    fn call(&self, ctx: &Context<CD>);
}

// The empty case: a handler with no params
// In practice, this would mostly be used for debugging,
// or returning static values as a pure view external function
impl<F, CD> Handler<((),), CD> for F
where
    F: Fn(),
{
    fn call(&self, _ctx: &Context<CD>) {
        self()
    }
}

// I've manually impl'ed Handler for 0, 1, or 2 params
// We'll want to extract this into a macro for up to max extractor params similar to:
// https://github.com/tokio-rs/axum/blob/main/axum/src/handler/mod.rs#L181
// https://github.com/lunatic-solutions/submillisecond/blob/main/src/handler.rs#L138
impl<F, T1, CD> Handler<((), T1), CD> for F
where
    F: Fn(&T1),
    T1: FromContext<CD>,
{
    fn call(&self, ctx: &Context<CD>) {
        self(T1::from_context(&ctx));
    }
}

// Two param handler (allows for two extractors to be defined)
impl<F, T1, T2, CD> Handler<((), T1, T2), CD> for F
where
    F: Fn(&T1, &T2),
    T1: FromContext<CD>,
    T2: FromContext<CD>,
{
    fn call(&self, ctx: &Context<CD>) {
        self(T1::from_context(&ctx), T2::from_context(&ctx));
    }
}

// given a handler, calls call() on that handler
pub fn trigger<T, H, CD>(ctx: &Context<CD>, handler: H)
where
    H: Handler<T, CD>,
{
    handler.call(&ctx);
}
