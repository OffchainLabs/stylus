use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Params)]
pub fn derive_from_context(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // used in quasi-quotation below as #name
    let name = input.ident;

    let expanded = quote! {
      impl arbitrum::extractors::FromContext<#name> for #name {
        fn from_context(ctx: &arbitrum::extractors::Context<#name>) -> &Self {
          &ctx.calldata.0
        }
      }
    };

    proc_macro::TokenStream::from(expanded)
}
