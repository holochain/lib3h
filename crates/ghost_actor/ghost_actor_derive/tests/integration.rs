#[macro_use]
extern crate ghost_actor_derive;

mod test_proto {
    #[derive(Debug, Clone, PartialEq)]
    pub struct Print(pub String);
    #[derive(Debug, Clone, PartialEq)]
    pub struct Add1(pub i32);

    ghost_protocol! {
        prefix(test_proto),
        event_to_actor(print, Print),
        request_to_actor(add_1, Add1, Result<Add1, ()>),
        event_to_owner(print, Print),
        request_to_owner(add_1, Add1, Result<Add1, ()>),
    }
}

#[test]
fn it_should_be_usable() {
    let p =
        test_proto::TestProtoProtocol::EventToActorPrint(test_proto::Print("testing".to_string()));
    assert_eq!("EventToActorPrint(Print(\"testing\"))", &format!("{:?}", p));
}
