
#[cfg(test)]
mod tests {
    use crate::*;
    use crate::test_proto::*;

    struct TActor {

    }

    impl<'lt> TestProtoActorTarget<'lt, TActor> for TActor {
        fn send_protocol<'a>(&'a mut self, message: TestProtoProtocol, _cb: Option<GhostResponseCb<'lt, TActor, TestProtoProtocol>>) -> GhostResult<()> {
            println!("{:?}", message);
            Ok(())
        }
    }

    impl<'lt> TestProtoActorTargetHandle<'lt, TActor> for TActor {
        fn handle<'a, H: TestProtoActorHandler<'a>>(&mut self, _handler: &'a mut H) -> GhostResult<()> {
            Ok(())
        }
    }

    #[test]
    fn it_should_run_actor() {
        let a = TestProtoProtocol::EventToActorPrint(Print("zombies".to_string()));
        assert_eq!(
            "EventToActorPrint(Print(\"zombies\"))",
            &format!("{:?}", a),
        );
    }
}
