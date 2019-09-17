#[cfg(test)]
mod tests {
    use crate::{test_proto::*, *};
    //use super::*;

    struct TActor {}

    impl<'lt> TestProtoActorHandler<'lt> for TActor {
        fn handle_event_to_actor_print(&mut self, message: Print) -> GhostResult<()> {
            println!("GOT: {:?}", message);
            Ok(())
        }

        fn handle_request_to_actor_add_1(
            &mut self,
            message: Add1,
            cb: GhostHandlerCb<'lt, Result<Add1, ()>>,
        ) -> GhostResult<()> {
            cb(Ok(Add1(message.0 + 1)))
        }
    }

    pub struct TActorFull<'lt, A: 'lt + TestProtoActorHandler<'lt>> {
        _inner: std::sync::Arc<std::sync::RwLock<A>>,
        phantom_lifetime: std::marker::PhantomData<&'lt i8>,
    }

    impl<'lt, A: 'lt + TestProtoActorHandler<'lt>> TActorFull<'lt, A> {
        pub fn new(inner: A) -> Self {
            Self {
                _inner: std::sync::Arc::new(std::sync::RwLock::new(inner)),
                phantom_lifetime: std::marker::PhantomData,
            }
        }
    }

    impl<'lt, A: 'lt + TestProtoActorHandler<'lt>> TestProtoActorTarget<'lt, TActorFull<'lt, A>>
        for TActorFull<'lt, A>
    {
        fn send_protocol<'a>(
            &'a mut self,
            message: TestProtoProtocol,
            _cb: Option<GhostResponseCb<'lt, TActorFull<'lt, A>, TestProtoProtocol>>,
        ) -> GhostResult<()> {
            println!("{:?}", message);
            Ok(())
        }
    }

    impl<'lt, A: 'lt + TestProtoActorHandler<'lt>>
        TestProtoActorTargetHandle<'lt, TActorFull<'lt, A>> for TActorFull<'lt, A>
    {
        fn handle<'a, H: TestProtoActorHandler<'a>>(
            &mut self,
            _handler: &'a mut H,
        ) -> GhostResult<()> {
            Ok(())
        }
    }

    /*
    pub struct TOwnerFull<'lt, X: 'lt> {
        phantom_lifetime: std::marker::PhantomData<&'lt i8>,
    }

    impl<'lt, X: 'lt> TestProtoOwnerTarget<'lt, X> for
    */

    #[test]
    fn it_should_run_actor() {
        let a = TestProtoProtocol::EventToActorPrint(Print("zombies".to_string()));
        assert_eq!("EventToActorPrint(Print(\"zombies\"))", &format!("{:?}", a),);
        let _ = TActorFull::new(TActor {});
    }
}
