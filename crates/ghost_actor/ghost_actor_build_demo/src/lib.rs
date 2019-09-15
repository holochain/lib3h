extern crate ghost_actor;

pub mod test_proto {
    include!(concat!(env!("OUT_DIR"), "/test_proto.rs"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_be_usable() {
        let p = test_proto::TestProtoProtocol::EventToActorPrint(test_proto::Print(
            "testing".to_string(),
        ));
        assert_eq!("EventToActorPrint(Print(\"testing\"))", &format!("{:?}", p));
    }
}
