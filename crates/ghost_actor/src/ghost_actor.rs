use crate::{
    GhostCallback, GhostContextEndpoint, GhostEndpoint, GhostMessage, GhostResult, WorkWasDone,
};
use std::any::Any;

/// helper struct that merges (on the parent side) the actual child
/// GhostActor instance, with the child's ghost channel endpoint.
/// You only have to call process() on this one struct, and it provides
/// all the request / drain_messages etc functions from GhostEndpoint.
pub struct GhostParentWrapper<
    UserData,
    Context,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    Error,
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
        Context,
        RequestToChild,
        RequestToChildResponse,
        RequestToParent,
        RequestToParentResponse,
        Error,
    >,
}

impl<
        UserData: 'static,
        Context: 'static,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
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
        Context,
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
            .as_context_endpoint(request_id_prefix);
        Self { actor, endpoint }
    }

    /// see GhostContextEndpoint::publish
    pub fn publish(&mut self, payload: RequestToChild) {
        self.endpoint.publish(payload)
    }

    /// see GhostContextEndpoint::request
    pub fn request(
        &mut self,
        timeout: std::time::Duration,
        context: Context,
        payload: RequestToChild,
        cb: GhostCallback<UserData, Context, RequestToChildResponse, Error>,
    ) {
        self.endpoint.request(timeout, context, payload, cb)
    }

    /// see GhostContextEndpoint::drain_messages
    pub fn drain_messages(
        &mut self,
    ) -> Vec<GhostMessage<RequestToParent, RequestToChild, RequestToParentResponse, Error>> {
        self.endpoint.drain_messages()
    }

    /// see GhostContextEndpoint::process and GhostActor::process
    pub fn process(&mut self, user_data: &mut UserData) -> GhostResult<()> {
        self.actor.process()?;
        self.endpoint.process(user_data)?;
        Ok(())
    }
}

impl<
        UserData,
        Context,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
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
        Context,
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
        Context,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
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
        Context,
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

pub trait GhostActor<
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    Error,
>
{
    /// get a generic reference to ourselves
    /// will be passed into any endpoint process functions
    fn as_any(&mut self) -> &mut dyn Any;

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

/// same as above, but takes a trait object child
pub struct GhostParentWrapperDyn<
    UserData,
    Context,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    Error,
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
        Context,
        RequestToChild,
        RequestToChildResponse,
        RequestToParent,
        RequestToParentResponse,
        Error,
    >,
}

impl<
        UserData,
        Context: 'static,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse: 'static,
        Error: 'static,
    >
    GhostParentWrapperDyn<
        UserData,
        Context,
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
        let endpoint = actor
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint(request_id_prefix);
        Self { actor, endpoint }
    }

    /// see GhostContextEndpoint::publish
    pub fn publish(&mut self, payload: RequestToChild) {
        self.endpoint.publish(payload)
    }

    /// see GhostContextEndpoint::request
    pub fn request(
        &mut self,
        timeout: std::time::Duration,
        context: Context,
        payload: RequestToChild,
        cb: GhostCallback<UserData, Context, RequestToChildResponse, Error>,
    ) {
        self.endpoint.request(timeout, context, payload, cb)
    }

    /// see GhostContextEndpoint::drain_messages
    pub fn drain_messages(
        &mut self,
    ) -> Vec<GhostMessage<RequestToParent, RequestToChild, RequestToParentResponse, Error>> {
        self.endpoint.drain_messages()
    }

    /// see GhostContextEndpoint::process and GhostActor::process
    pub fn process(&mut self, user_data: &mut UserData) -> GhostResult<()> {
        self.actor.process()?;
        self.endpoint.process(user_data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ghost_channel::create_ghost_channel, ghost_tracker::GhostCallbackData};
    use detach::prelude::*;
    use std::any::Any;

    // Any actor has messages that it exchanges with it's parent
    // These are the Out message, and it has messages that come internally
    // either self-generated or (presumeably) from children
    #[derive(Debug)]
    struct TestMsgOut(String);
    #[derive(Debug)]
    struct TestMsgOutResponse(String);
    #[derive(Debug)]
    struct TestMsgIn(String);
    #[derive(Debug)]
    struct TestMsgInResponse(String);
    type TestError = String;
    #[derive(Debug)]
    struct TestContext(String);

    struct TestActor {
        endpoint_for_parent: Option<
            GhostEndpoint<TestMsgIn, TestMsgInResponse, TestMsgOut, TestMsgOutResponse, TestError>,
        >,
        endpoint_as_child: Detach<
            GhostContextEndpoint<
                TestActor,
                String,
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
                endpoint_as_child: Detach::new(endpoint_self.as_context_endpoint("child")),
                internal_state: Vec::new(),
            }
        }
    }

    impl GhostActor<TestMsgOut, TestMsgOutResponse, TestMsgIn, TestMsgInResponse, TestError>
        for TestActor
    {
        // START BOILER PLATE--------------------------
        fn as_any(&mut self) -> &mut dyn Any {
            &mut *self
        }

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
            // START BOILER PLATE--------------------------
            // always run the endpoint process loop
            detach_run!(&mut self.endpoint_as_child, |cs| { cs.process(self) })?;
            // END BOILER PLATE--------------------------

            // In this test actor we simply take all the messages we get and
            // add them to our internal state.
            for mut msg in self.endpoint_as_child.as_mut().drain_messages() {
                let payload = match msg.take_message().expect("exists") {
                    TestMsgIn(payload) => payload,
                };
                self.internal_state.push(payload.clone());
                if msg.is_request() {
                    msg.respond(Ok(TestMsgInResponse(format!("we got: {}", payload))))
                };
            }
            Ok(false.into())
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
            TestContext,
            TestMsgIn,
            TestMsgInResponse,
            TestMsgOut,
            TestMsgOutResponse,
            TestError,
        > = child_actor
            .take_parent_endpoint()
            .unwrap()
            .as_context_endpoint("parent");

        // now lets post an event from the parent
        parent_endpoint.publish(TestMsgIn("event from parent".into()));

        // now process the events on the child and watch that internal state has chaned
        assert!(child_actor.process().is_ok());
        assert_eq!(
            "\"event from parent\"",
            format!("{:?}", child_actor.internal_state[0])
        );

        // now lets try posting a request with a callback which just saves the response
        // value to the parent's statee
        let cb: GhostCallback<FakeParent, TestContext, TestMsgInResponse, TestError> =
            Box::new(|parent, _context, callback_data| {
                if let GhostCallbackData::Response(Ok(TestMsgInResponse(payload))) = callback_data {
                    parent.state = payload;
                }
                Ok(())
            });

        parent_endpoint.request(
            std::time::Duration::from_millis(1000),
            TestContext("context data".into()),
            TestMsgIn("event from parent".into()),
            cb,
        );
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
            TestContext,
            TestMsgOut,
            TestMsgOutResponse,
            TestMsgIn,
            TestMsgInResponse,
            TestError,
            TestActor,
        > = GhostParentWrapper::new(TestActor::new(), "parent");

        // use it to publish an event via the wrapper
        wrapped_child.publish(TestMsgIn("event from parent".into()));

        // process via the wrapper
        assert!(wrapped_child.process(&mut fake_parent).is_ok());

        assert_eq!(
            "\"event from parent\"",
            format!("{:?}", wrapped_child.as_ref().internal_state[0])
        )
    }

}
