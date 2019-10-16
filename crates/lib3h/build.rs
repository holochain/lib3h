extern crate ghost_actor;
extern crate proc_macro2;
#[macro_use]
extern crate quote;

use proc_macro2::TokenStream;
use std::io::Write;

/// to get debug output in a build.rs script every line needs a prefix
fn debug(s: &str) {
    let s = s.replace("\n", "\ncargo:warning=");
    println!("cargo:warning={}", &s);
}

/// this will eventually use the ghost_actor code generation to render a protocol
fn render_protocol(input: TokenStream) -> TokenStream {
    // some day we need to actually render this
    // for now, we are just passing it through
    input
}

/// write a token stream to an output file
fn write_file(file_name: &str, content: TokenStream) {
    let mut dest = std::path::PathBuf::new();
    dest.push(std::env::var("OUT_DIR").unwrap());
    dest.push(file_name);

    let rendered = match ghost_actor::code_gen::try_fmt(content.to_string()) {
        // formatting isn't strictly necessary...
        // but for now, let us panic! on formatting errors
        // because it fails fast if we have hand-coded syntax errors
        Err(e) => panic!(e),
        Ok(r) => r,
    };

    debug(&format!("-- begin file {:?} --", dest));
    debug(&rendered);
    debug(&format!("--  end  file {:?} --", dest));

    let mut file = std::fs::File::create(&dest).unwrap();
    file.write_all(rendered.as_bytes()).unwrap();
}

/// main entrypoint
fn main() {
    write_file(
        "keystore_protocol.rs",
        render_protocol(get_keystore_protocol()),
    );
}

/// helper to get the keystore protocol definition
/// (right now this gets the actual full content)
fn get_keystore_protocol() -> TokenStream {
    quote! {
        // -- first, the data types -- //

        ///We need to support both agent pubkeys and node pubkeys
        pub enum PubKey {
            AgentPubKey(::lib3h_protocol::types::AgentPubKey),
            NodePubKey(::lib3h_protocol::types::NodePubKey),
        }

        ///Data required for submitting a keystore signature request
        pub struct SignRequestData {
            pub pub_key: PubKey,
            pub data: ::lib3h_protocol::data_types::Opaque,
        }

        ///A successful result of a keystore signature request
        pub struct SignResultData {
            pub signature: ::lib3h_protocol::data_types::Opaque,
        }

        // Once we have code generation working
        // we should only need something like the folowing:
        //
        // -- begin definition --
        // name(keystore),
        // request_to_actor(sign, SignRequestData, Result<SignResultData, Lib3hError>),
        // -- end definition --

        ///Hand-Rolled KeystoreProtocol Enum
        pub enum KeystoreProtocol {
            RequestToActorSign(SignRequestData),
            RequestToActorSignResponse(Result<SignResultData, crate::error::Lib3hError>),
        }
    }
}
