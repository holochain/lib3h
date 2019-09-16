use crate::prelude::*;
use lib3h_tracing::Lib3hSpan; //--------------------------------------------------------------------------------------------------
                              // GhostParentWrapper
                              //---------------------------------------------------------------------------------------------------

/// helper struct that merges (on the parent side) the actual child
/// GhostActor instance, with the child's ghost channel endpoint.
/// You only have to call process() on this one struct, and it provides
/// all the request / drain_messages etc functions from GhostEndpoint.
pub struct GhostParentWrapper<
    UserData,
    RequestToParent: 'static,
    RequestToParentResponse: 'static,
    RequestToChild: 'static,
    RequestToChildResponse: 'static,
    Error: 'static,
    Actor: GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
    >,
> {
    actor: Actor,
    endpoint: GhostContextEndpoint<
        UserData,
        RequestToChild,
        RequestToChildResponse,
        RequestToParent,
        RequestToParentResponse,
        Error,
    >,
}

impl<
        UserData,
        RequestToParent: 'static,
        RequestToParentResponse: 'static,
        RequestToChild: 'static,
        RequestToChildResponse: 'static,
        Error: 'static,
        Actor: GhostActor<
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            Error,
        >,
    >
    GhostParentWrapper<
        UserData,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
        Actor,
    >
{
    /// wrap a GhostActor instance and it's parent channel endpoint.
    pub fn new(mut actor: Actor, request_id_prefix: &str) -> Self {
        let endpoint = actor
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix(request_id_prefix)
            .build();
        Self { actor, endpoint }
    }
}

impl<
        UserData,
        RequestToParent: 'static,
        RequestToParentResponse: 'static,
        RequestToChild: 'static,
        RequestToChildResponse: 'static,
        Error: 'static,
        Actor: GhostActor<
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            Error,
        >,
    >
    GhostCanTrack<
        UserData,
        RequestToChild,
        RequestToChildResponse,
        RequestToParent,
        RequestToParentResponse,
        Error,
    >
    for GhostParentWrapper<
        UserData,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
        Actor,
    >
{
    /// see GhostContextEndpoint::publish
    fn publish(&mut self, span: Lib3hSpan, payload: RequestToChild) -> GhostResult<()> {
        self.endpoint.publish(span, payload)
    }

    /// see GhostContextEndpoint::request
    fn request(
        &mut self,
        span: Lib3hSpan,
        payload: RequestToChild,
        cb: GhostCallback<UserData, RequestToChildResponse, Error>,
    ) -> GhostResult<()> {
        self.endpoint.request(span, payload, cb)
    }

    /// see GhostContextEndpoint::request
    fn request_options(
        &mut self,
        span: Lib3hSpan,
        payload: RequestToChild,
        cb: GhostCallback<UserData, RequestToChildResponse, Error>,
        options: GhostTrackRequestOptions,
    ) -> GhostResult<()> {
        self.endpoint.request_options(span, payload, cb, options)
    }

    /// see GhostContextEndpoint::drain_messages
    fn drain_messages(
        &mut self,
    ) -> Vec<GhostMessage<RequestToParent, RequestToChild, RequestToParentResponse, Error>> {
        self.endpoint.drain_messages()
    }

    /// see GhostContextEndpoint::process and GhostActor::process
    fn process(&mut self, user_data: &mut UserData) -> GhostResult<WorkWasDone> {
        let mut work_was_done = self.actor.process()?;
        work_was_done = work_was_done.or(self.endpoint.process(user_data)?);
        Ok(work_was_done)
    }
}

impl<
        UserData,
        RequestToParent: 'static,
        RequestToParentResponse: 'static,
        RequestToChild: 'static,
        RequestToChildResponse: 'static,
        Error: 'static,
        Actor: GhostActor<
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            Error,
        >,
    > std::convert::AsRef<Actor>
    for GhostParentWrapper<
        UserData,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
        Actor,
    >
{
    fn as_ref(&self) -> &Actor {
        &self.actor
    }
}

impl<
        UserData,
        RequestToParent: 'static,
        RequestToParentResponse: 'static,
        RequestToChild: 'static,
        RequestToChildResponse: 'static,
        Error: 'static,
        Actor: GhostActor<
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            Error,
        >,
    > std::convert::AsMut<Actor>
    for GhostParentWrapper<
        UserData,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
        Actor,
    >
{
    fn as_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
}

//--------------------------------------------------------------------------------------------------
// GhostActor
//---------------------------------------------------------------------------------------------------

pub trait GhostActor<
    RequestToParent: 'static,
    RequestToParentResponse: 'static,
    RequestToChild: 'static,
    RequestToChildResponse: 'static,
    Error: 'static,
>
{
    /// our parent gets a reference to the parent side of our channel
    fn take_parent_endpoint(
        &mut self,
    ) -> Option<
        GhostEndpoint<
            RequestToChild,
            RequestToChildResponse,
            RequestToParent,
            RequestToParentResponse,
            Error,
        >,
    >;

    /// our parent will call this process function
    fn process(&mut self) -> GhostResult<WorkWasDone> {
        // it would be awesome if this trait level could handle things like:
        //  `self.endpoint_self.process();`
        self.process_concrete()
    }

    /// we, as a ghost actor implement this, it will get called from
    /// process after the subconscious process items have run
    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        Ok(false.into())
    }
}

//--------------------------------------------------------------------------------------------------
// GhostParentWrapperDyn
//---------------------------------------------------------------------------------------------------

/// same as above, but takes a trait object child
pub struct GhostParentWrapperDyn<
    UserData,
    RequestToParent: 'static,
    RequestToParentResponse: 'static,
    RequestToChild: 'static,
    RequestToChildResponse: 'static,
    Error: 'static,
> {
    actor: Box<
        dyn GhostActor<
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            Error,
        >,
    >,
    endpoint: GhostContextEndpoint<
        UserData,
        RequestToChild,
        RequestToChildResponse,
        RequestToParent,
        RequestToParentResponse,
        Error,
    >,
}

impl<
        UserData,
        RequestToParent: 'static,
        RequestToParentResponse: 'static,
        RequestToChild: 'static,
        RequestToChildResponse: 'static,
        Error: 'static,
    >
    GhostParentWrapperDyn<
        UserData,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
    >
{
    /// wrap a GhostActor instance and it's parent channel endpoint.
    pub fn new(
        mut actor: Box<
            dyn GhostActor<
                RequestToParent,
                RequestToParentResponse,
                RequestToChild,
                RequestToChildResponse,
                Error,
            >,
        >,
        request_id_prefix: &str,
    ) -> Self {
        let endpoint: GhostContextEndpoint<UserData, _, _, _, _, _> = actor
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix(request_id_prefix)
            .build();
        Self { actor, endpoint }
    }
}

impl<
        UserData,
        RequestToParent: 'static,
        RequestToParentResponse: 'static,
        RequestToChild: 'static,
        RequestToChildResponse: 'static,
        Error: 'static,
    >
    GhostCanTrack<
        UserData,
        RequestToChild,
        RequestToChildResponse,
        RequestToParent,
        RequestToParentResponse,
        Error,
    >
    for GhostParentWrapperDyn<
        UserData,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
    >
{
    /// see GhostContextEndpoint::publish
    fn publish(&mut self, span: Lib3hSpan, payload: RequestToChild) -> GhostResult<()> {
        self.endpoint.publish(span, payload)
    }

    /// see GhostContextEndpoint::request
    fn request(
        &mut self,
        span: Lib3hSpan,
        payload: RequestToChild,
        cb: GhostCallback<UserData, RequestToChildResponse, Error>,
    ) -> GhostResult<()> {
        self.endpoint.request(span, payload, cb)
    }

    fn request_options(
        &mut self,
        span: Lib3hSpan,
        payload: RequestToChild,
        cb: GhostCallback<UserData, RequestToChildResponse, Error>,
        options: GhostTrackRequestOptions,
    ) -> GhostResult<()> {
        self.endpoint.request_options(span, payload, cb, options)
    }

    /// see GhostContextEndpoint::drain_messages
    fn drain_messages(
        &mut self,
    ) -> Vec<GhostMessage<RequestToParent, RequestToChild, RequestToParentResponse, Error>> {
        self.endpoint.drain_messages()
    }

    /// see GhostContextEndpoint::process and GhostActor::process
    fn process(&mut self, user_data: &mut UserData) -> GhostResult<WorkWasDone> {
        let mut work_was_done = self.actor.process()?;
        work_was_done = work_was_done.or(self.endpoint.process(user_data)?);
        Ok(work_was_done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ghost_channel::create_ghost_channel, ghost_tracker::GhostCallbackData};
    use detach::prelude::*;
    use lib3h_tracing::test_span;
    //    use predicates::prelude::*;
    use crate::ghost_test_harness::*;

    type TestError = String;

    // Any actor has messages that it exchanges with it's parent
    // These are the Out message, and it has messages that come internally
    // either self-generated or (presumeably) from children
    #[derive(Debug, PartialEq)]
    struct TestMsgOut(String);
    #[derive(Debug, PartialEq)]
    struct TestMsgOutResponse(String);
    #[derive(Debug)]
    struct TestMsgIn(String);
    #[derive(Debug, PartialEq, Clone)]
    struct TestMsgInResponse(String);

    struct TestActor {
        endpoint_for_parent: Option<
            GhostEndpoint<TestMsgIn, TestMsgInResponse, TestMsgOut, TestMsgOutResponse, TestError>,
        >,
        endpoint_as_child: Detach<
            GhostContextEndpoint<
                TestActor,
                TestMsgOut,
                TestMsgOutResponse,
                TestMsgIn,
                TestMsgInResponse,
                TestError,
            >,
        >,
        internal_state: Vec<String>,
    }

    impl TestActor {
        pub fn new() -> Self {
            let (endpoint_parent, endpoint_self) = create_ghost_channel();
            Self {
                endpoint_for_parent: Some(endpoint_parent),
                endpoint_as_child: Detach::new(
                    endpoint_self
                        .as_context_endpoint_builder()
                        .request_id_prefix("child")
                        .build(),
                ),
                internal_state: Vec::new(),
            }
        }
    }

    impl GhostActor<TestMsgOut, TestMsgOutResponse, TestMsgIn, TestMsgInResponse, TestError>
        for TestActor
    {
        // START BOILER PLATE--------------------------

        fn take_parent_endpoint(
            &mut self,
        ) -> Option<
            GhostEndpoint<TestMsgIn, TestMsgInResponse, TestMsgOut, TestMsgOutResponse, TestError>,
        > {
            std::mem::replace(&mut self.endpoint_for_parent, None)
        }
        // END BOILER PLATE--------------------------

        // for this test actor what we do
        fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
            println!("process_concrete!");
            // START BOILER PLATE--------------------------
            // always run the endpoint process loop
            detach_run!(&mut self.endpoint_as_child, |cs| cs.process(self))?;
            // END BOILER PLATE--------------------------

            // In this test actor we simply take all the messages we get and
            // add them to our internal state.
            let mut did_work = false;
            for mut msg in self.endpoint_as_child.as_mut().drain_messages() {
                println!("process_concrete, got msg");
                let payload = match msg.take_message().expect("exists") {
                    TestMsgIn(payload) => payload,
                };
                self.internal_state.push(payload.clone());
                if msg.is_request() {
                    msg.respond(Ok(TestMsgInResponse(format!("we got: {}", payload))))?;
                };
                did_work |= true;
            }
            Ok(did_work.into())
        }
    }

    struct FakeParent {
        state: String,
    }

    #[test]
    fn test_ghost_actor() {
        // The body of this test simulates being the parent actor
        let mut fake_parent = FakeParent {
            state: "".to_string(),
        };

        // then we create the child actor
        let mut child_actor = TestActor::new();
        // get the endpoint from the child actor that we as parent will interact with
        let mut parent_endpoint: GhostContextEndpoint<
            FakeParent,
            TestMsgIn,
            TestMsgInResponse,
            TestMsgOut,
            TestMsgOutResponse,
            TestError,
        > = child_actor
            .take_parent_endpoint()
            .unwrap()
            .as_context_endpoint_builder()
            .request_id_prefix("parent")
            .build();

        let span = test_span("test_ghost_actor");

        // now lets post an event from the parent
        parent_endpoint
            .publish(span, TestMsgIn("event from parent".into()))
            .unwrap();

        // now process the events on the child and watch that internal state has chaned
        assert!(child_actor.process().is_ok());
        assert_eq!(
            "\"event from parent\"",
            format!("{:?}", child_actor.internal_state[0])
        );

        // now lets try posting a request with a callback which just saves the response
        // value to the parent's statee
        let cb: GhostCallback<FakeParent, TestMsgInResponse, TestError> =
            Box::new(|parent, callback_data| {
                if let GhostCallbackData::Response(Ok(TestMsgInResponse(payload))) = callback_data {
                    parent.state = payload;
                }
                Ok(())
            });

        parent_endpoint
            .request(
                test_span("context data"),
                TestMsgIn("event from parent".into()),
                cb,
            )
            .unwrap();
        assert!(child_actor.process().is_ok());
        assert!(parent_endpoint.process(&mut fake_parent).is_ok());
        assert_eq!("we got: event from parent", fake_parent.state);
    }

    #[test]
    fn test_ghost_actor_parent_wrapper() {
        // much of the previous test is the parent creating instances of the actor
        // and taking control of the parent endpoint.  Parent wrapper implements
        // much of this work as a convenience

        let mut fake_parent = FakeParent {
            state: "".to_string(),
        };

        // create the wrapper
        let mut wrapped_child: GhostParentWrapper<
            FakeParent,
            TestMsgOut,
            TestMsgOutResponse,
            TestMsgIn,
            TestMsgInResponse,
            TestError,
            TestActor,
        > = GhostParentWrapper::new(TestActor::new(), "parent");

        // use it to publish an event via the wrapper
        wrapped_child
            .publish(test_span(""), TestMsgIn("event from parent".into()))
            .unwrap();

        // process via the wrapper
        assert!(wrapped_child.process(&mut fake_parent).is_ok());

        assert_eq!(
            "\"event from parent\"",
            format!("{:?}", wrapped_child.as_ref().internal_state[0])
        )
    }

    #[test]
    #[ignore]
    fn test_ghost_actor_parent_wrapper_macro() {
        // much of the previous test is the parent creating instances of the actor
        // and taking control of the parent endpoint.  Parent wrapper implements
        // much of this work as a convenience

        let mut _fake_parent = FakeParent {
            state: "".to_string(),
        };

        // create the wrapper
        let mut _wrapped_child: GhostParentWrapper<
            FakeParent,
            TestMsgOut,
            TestMsgOutResponse,
            TestMsgIn,
            TestMsgInResponse,
            TestError,
            TestActor,
        > = GhostParentWrapper::new(TestActor::new(), "parent");

        // use it to publish an event via the wrapper
        let _test_msg_in = TestMsgIn("event from parent".into());

        let _test_msg_out = TestMsgOut("event from parent".into());

        //        assert_callback_eq!(wrapped_child, fake_parent, test_msg_in, test_msg_out, String);
    }

    #[test]
    fn test_callback_equals_as_processor_trait() {
        let callback_equals: CallbackDataEquals<TestMsgOut, _> =
            CallbackDataEquals(TestMsgOut("abc".into()), std::marker::PhantomData);
        let _as_processor: Box<dyn Processor<TestMsgOut, String>> = Box::new(callback_equals);
    }

}
