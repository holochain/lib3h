#[macro_use]
extern crate ghost_actor_derive;

mod proto {
    ghost_protocol! {
        #[derive(Debug)]
        /// Testing Protocol
        pub enum TestProtocol {
            TestProtocolVariant,
        }
    }
}

use proto::*;

#[test]
fn it_renders_ghost_protocol() {
    assert_eq!(
        "TestProtocolVariant",
        &format!("{:?}", TestProtocol::TestProtocolVariant),
    );
}
