extern crate proc_macro2;
#[macro_use]
extern crate quote;

use proc_macro2::TokenStream;
//use quote::ToTokens;
use std::io::Write;

#[allow(dead_code)]
#[allow(unused_imports)]
/// manually construct our library with include! macros
mod ghost_actor {
    include!("./src/ghost_error.rs");
    include!("./src/ghost_protocol.rs");
    include!("./src/ghost_system.rs");
    include!("./src/ghost_actor.rs");
    pub mod code_gen {
        include!("./src/code_gen/protocol.rs");
        include!("./src/code_gen/try_fmt.rs");
    }
}

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
        root(super),
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

    let types = types();
    let protocol = protocol();

    let rendered = quote! {
        #types
        #protocol
    };

    let rendered = ghost_actor::code_gen::try_fmt(rendered.to_string());

    {
        let mut file = std::fs::File::create(&dest).unwrap();
        file.write_all(rendered.as_bytes()).unwrap();
    }
}
