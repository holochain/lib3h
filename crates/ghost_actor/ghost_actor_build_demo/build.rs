extern crate ghost_actor;
extern crate proc_macro2;
#[macro_use]
extern crate quote;

use proc_macro2::TokenStream;
use quote::ToTokens;
use std::io::Write;

#[allow(dead_code)]
fn debug(s: &str) {
    let s = s.to_string();
    let s = s.replace("\n", "\ncargo:warning=");
    println!("cargo:warning={}", &s);
}

fn types() -> TokenStream {
    quote! {
        #[derive(Debug, Clone, PartialEq)]
        pub struct Print(pub String);
        #[derive(Debug, Clone, PartialEq)]
        pub struct Add1(pub i32);
    }
}

fn protocol() -> ghost_actor::code_gen::Protocol {
    ghost_actor::code_gen::Protocol::new(quote! {
        prefix(test_proto),
        event_to_actor(print, Print),
        request_to_actor(add_1, Add1, Result<Add1, ()>),
        event_to_owner(print, Print),
        request_to_owner(add_1, Add1, Result<Add1, ()>),
    })
}

fn main() {
    let mut dest = std::path::PathBuf::new();
    dest.push(std::env::var("OUT_DIR").unwrap());
    dest.push("test_proto.rs");
    //debug(&format!("{:#?}", std::env::var("OUT_DIR")));
    //debug(dest.to_str().unwrap());

    let types = types();
    let protocol = protocol();

    let mut rendered = types.into_token_stream();
    rendered.extend(protocol.into_token_stream());

    let rendered = ghost_actor::code_gen::try_fmt(rendered.to_string());

    //debug(&rendered);

    {
        let mut file = std::fs::File::create(&dest).unwrap();
        file.write_all(rendered.as_bytes()).unwrap();
    }
}
