#[macro_use]
extern crate ghost_actor_derive;

mod proto {
    ghost_protocol! {}
}

use proto::*;

#[test]
fn it_renders_ghost_protocol() {
    assert_eq!(
        "TestVariant",
        &format!("{:?}", TestEnum::TestVariant),
    );
}
