#![recursion_limit = "128"]
extern crate ghost_actor;
extern crate proc_macro;
extern crate quote;

use proc_macro::TokenStream;
use quote::ToTokens;

#[proc_macro]
pub fn ghost_protocol(tokens: TokenStream) -> TokenStream {
    let protocol = ghost_actor::code_gen::Protocol::new(tokens.into());
    protocol.into_token_stream().into()
}
