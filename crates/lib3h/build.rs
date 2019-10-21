extern crate ghost_actor;
extern crate proc_macro2;
#[macro_use]
extern crate quote;

use proc_macro2::TokenStream;
use std::io::Write;

/// to get debug output in a build.rs script every line needs a prefix
fn debug(s: &str) {
    if std::env::var("GHOST_DEBUG_CODEGEN").is_ok() {
        let s = s.replace("\n", "\ncargo:warning=");
        println!("cargo:warning={}", &s);
    }
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
        #[derive(Debug, Clone)]
        pub enum PubKey {
            AgentPubKey(::lib3h_protocol::types::AgentPubKey),
            NodePubKey(::lib3h_protocol::types::NodePubKey),
        }

        ///Data required for submitting a keystore signature request
        #[derive(Debug, Clone)]
        pub struct SignRequestData {
            pub pub_key: PubKey,
            pub data: ::lib3h_protocol::data_types::Opaque,
        }

        ///A successful result of a keystore signature request
        #[derive(Debug, Clone)]
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

        // all code below will go away once we implement code generation:

        ///Hand-Rolled KeystoreProtocol Enum
        #[derive(Debug, Clone)]
        pub enum KeystoreProtocol {
            RequestToActorSign(SignRequestData),
            RequestToActorSignResponse(Result<SignResultData, crate::error::Lib3hError>),
        }

        static D_LIST: &'static [::ghost_actor::GhostProtocolDiscriminant] = &[
            ::ghost_actor::GhostProtocolDiscriminant {
                id: "request_to_actor_sign",
                destination: ::ghost_actor::GhostProtocolDestination::Actor,
                variant_type: ::ghost_actor::GhostProtocolVariantType::Request,
            },
            ::ghost_actor::GhostProtocolDiscriminant {
                id: "request_to_actor_sign_response",
                destination: ::ghost_actor::GhostProtocolDestination::Owner,
                variant_type: ::ghost_actor::GhostProtocolVariantType::Response,
            },
        ];

        impl ::ghost_actor::GhostProtocol for KeystoreProtocol {
            fn discriminant_list() -> &'static [::ghost_actor::GhostProtocolDiscriminant] {
                D_LIST
            }

            fn discriminant(&self) -> &::ghost_actor::GhostProtocolDiscriminant {
                match self {
                    KeystoreProtocol::RequestToActorSign(_) => &D_LIST[0],
                    KeystoreProtocol::RequestToActorSignResponse(_) => &D_LIST[1],
                }
            }
        }

        ///Implement this to handle messages sent to a keystore actor
        pub struct KeystoreActorHandler<'lt, X: 'lt + Send + Sync> {
            pub handle_request_to_actor_sign: Box<
                dyn FnMut(&mut X, SignRequestData, ::ghost_actor::GhostHandlerCb<'lt, Result<SignResultData, crate::error::Lib3hError>>) -> ::ghost_actor::GhostResult<()> + 'lt + Send + Sync
            >,
        }

        impl<'lt, X: 'lt + Send + Sync> ::ghost_actor::GhostHandler<'lt, X, KeystoreProtocol> for KeystoreActorHandler<'lt, X> {
            fn trigger(
                &mut self,
                user_data: &mut X,
                message: KeystoreProtocol,
                cb: Option<::ghost_actor::GhostHandlerCb<'lt, KeystoreProtocol>>,
            ) -> ::ghost_actor::GhostResult<()> {
                match message {
                    KeystoreProtocol::RequestToActorSign(m) => {
                        let cb = cb.unwrap();
                        let cb = Box::new(move |resp| cb(KeystoreProtocol::RequestToActorSignResponse(resp)));
                        (self.handle_request_to_actor_sign)(user_data, m, cb)
                    }
                    _ => panic!("bad"),
                }
            }
        }

        ///Implement this to handle messages sent to the owner of a keystore
        pub struct KeystoreOwnerHandler<'lt, X: 'lt + Send + Sync> {
            pub phantom: ::std::marker::PhantomData<&'lt X>,
        }

        impl<'lt, X: 'lt + Send + Sync> ::ghost_actor::GhostHandler<'lt, X, KeystoreProtocol> for KeystoreOwnerHandler<'lt, X> {
            fn trigger(
                &mut self,
                _user_data: &mut X,
                _message: KeystoreProtocol,
                _cb: Option<::ghost_actor::GhostHandlerCb<'lt, KeystoreProtocol>>,
            ) -> ::ghost_actor::GhostResult<()> {
                panic!("bad")
            }
        }

        ///This will be implemented on GhostEndpointFull so you can easily emit
        ///events and issue requests / handle responses
        pub trait KeystoreActorRef<'lt, X: 'lt + Send + Sync>: ::ghost_actor::GhostEndpoint<'lt, X, KeystoreProtocol> {
            fn request_to_actor_sign(
                &mut self,
                message: SignRequestData,
                cb: ::ghost_actor::GhostResponseCb<'lt, X, Result<SignResultData, crate::error::Lib3hError>>,
            ) -> ::ghost_actor::GhostResult<()> {
                let cb: ::ghost_actor::GhostResponseCb<'lt, X, KeystoreProtocol> = Box::new(move |me, resp| {
                    cb(
                        me,
                        match resp {
                            Ok(r) => match r {
                                KeystoreProtocol::RequestToActorSignResponse(m) => Ok(m),
                                _ => panic!("bad"),
                            },
                            Err(e) => Err(e),
                        },
                    )
                });
                self.send_protocol(KeystoreProtocol::RequestToActorSign(message), Some(cb))
            }
        }

        impl<
            'lt,
            X: 'lt + Send + Sync,
            A: 'lt,
            H: ::ghost_actor::GhostHandler<'lt, X, KeystoreProtocol>,
            S: ::ghost_actor::GhostSystemRef<'lt>,
        > KeystoreActorRef<'lt, X> for ::ghost_actor::GhostEndpointFull<'lt, KeystoreProtocol, A, X, H, S> {}

        ///This will be implemented on GhostEndpointFull so you can easily emit
        ///events and issue requests / handle responses
        pub trait KeystoreOwnerRef<'lt, X: 'lt + Send + Sync>: ::ghost_actor::GhostEndpoint<'lt, X, KeystoreProtocol> {}

        impl<
            'lt,
            X: 'lt + Send + Sync,
            A: 'lt,
            H: ::ghost_actor::GhostHandler<'lt, X, KeystoreProtocol>,
            S: ::ghost_actor::GhostSystemRef<'lt>,
        > KeystoreOwnerRef<'lt, X> for ::ghost_actor::GhostEndpointFull<'lt, KeystoreProtocol, A, X, H, S> {}
    }
}
