use crate::{GhostCallback, GhostResult, GhostTracker, RequestId};
use std::any::Any;

/// enum used internally as the protocol for our crossbeam_channels
/// allows us to be explicit about which messages are requests or responses.
#[derive(Debug)]
enum GhostEndpointMessage<Request, Response, Error> {
    Request {
        request_id: Option<RequestId>,
        payload: Request,
    },
    Response {
        request_id: RequestId,
        payload: Result<Response, Error>,
    },
}

/// GhostContextEndpoints allow you to drain these incoming `GhostMessage`s
/// A GhostMessage contains the incoming request, as well as a hook to
/// allow a response to automatically be returned.
pub struct GhostMessage<MessageToSelf, MessageToOther, MessageToSelfResponse, Error> {
    request_id: Option<RequestId>,
    message: Option<MessageToSelf>,
    sender: crossbeam_channel::Sender<
        GhostEndpointMessage<MessageToOther, MessageToSelfResponse, Error>,
    >,
}

impl<RequestToSelf, RequestToOther, RequestToSelfResponse, Error: 'static> std::fmt::Debug
    for GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GhostMessage {{request_id: {:?}, ..}}", self.request_id)
    }
}

impl<RequestToSelf, RequestToOther, RequestToSelfResponse, Error: 'static>
    GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>
{
    fn new(
        request_id: Option<RequestId>,
        message: RequestToSelf,
        sender: crossbeam_channel::Sender<
            GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
        >,
    ) -> Self {
        Self {
            request_id,
            message: Some(message),
            sender,
        }
    }

    /// create a request message
    #[allow(dead_code)]
    fn new_request(
        request_id: RequestId,
        message: RequestToSelf,
        sender: crossbeam_channel::Sender<
            GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
        >,
    ) -> Self {
        GhostMessage::new(Some(request_id), message, sender)
    }

    /// create an event message
    #[allow(dead_code)]
    fn new_event(
        message: RequestToSelf,
        sender: crossbeam_channel::Sender<
            GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
        >,
    ) -> Self {
        GhostMessage::new(None, message, sender)
    }

    /// most often you will want to consume the contents of the request
    /// using take prevents a clone
    pub fn take_message(&mut self) -> Option<RequestToSelf> {
        std::mem::replace(&mut self.message, None)
    }

    /// send a response back to the origin of this request
    pub fn respond(self, payload: Result<RequestToSelfResponse, Error>) {
        if let Some(request_id) = &self.request_id {
            self.sender
                .send(GhostEndpointMessage::Response {
                    request_id: request_id.clone(),
                    payload,
                })
                .expect("should send");
        }
    }

    pub fn is_request(&self) -> bool {
        self.request_id.is_some()
    }
}

/// `create_ghost_channel` outputs two endpoints,
/// a parent_endpoint, and a child_endpoint
/// these raw endpoints are not very useful on their own. When you get them
/// to the place they will be used, you probably want to call
/// `as_context_endpoint()` on them.
pub struct GhostEndpoint<
    RequestToOther,
    RequestToOtherResponse,
    RequestToSelf,
    RequestToSelfResponse,
    Error,
> {
    sender: crossbeam_channel::Sender<
        GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
    >,
    receiver: crossbeam_channel::Receiver<
        GhostEndpointMessage<RequestToSelf, RequestToOtherResponse, Error>,
    >,
}

impl<
        RequestToOther,
        RequestToOtherResponse: 'static,
        RequestToSelf,
        RequestToSelfResponse,
        Error: 'static,
    >
    GhostEndpoint<
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
    >
{
    /// internal new, used by `create_ghost_channel()`
    fn new(
        sender: crossbeam_channel::Sender<
            GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
        >,
        receiver: crossbeam_channel::Receiver<
            GhostEndpointMessage<RequestToSelf, RequestToOtherResponse, Error>,
        >,
    ) -> Self {
        Self { sender, receiver }
    }

    /// expand a raw endpoint into something usable.
    /// <Context> let's you store data with individual `request` calls
    /// that will be available again when the callback is invoked.
    /// Feel free to use `as_context_endpoint::<()>("prefix")` if you
    /// don't need any context.
    /// request_id_prefix is a debugging hint... the request_ids generated
    /// for tracking request/response pairs will be prepended with this prefix.
    pub fn as_context_endpoint<Context: 'static>(
        self,
        request_id_prefix: &str,
    ) -> GhostContextEndpoint<
        Context,
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
    > {
        GhostContextEndpoint::new(request_id_prefix, self.sender, self.receiver)
    }
}

/// an expanded endpoint usable to send/receive requests/responses/events
/// see `GhostEndpoint::as_context_endpoint` for additional details
pub struct GhostContextEndpoint<
    Context,
    RequestToOther,
    RequestToOtherResponse,
    RequestToSelf,
    RequestToSelfResponse,
    Error,
> {
    sender: crossbeam_channel::Sender<
        GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
    >,
    receiver: crossbeam_channel::Receiver<
        GhostEndpointMessage<RequestToSelf, RequestToOtherResponse, Error>,
    >,
    pending_responses_tracker: GhostTracker<Context, RequestToOtherResponse, Error>,
    outbox_messages_to_self:
        Vec<GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>>,
}

impl<
        Context: 'static,
        RequestToOther,
        RequestToOtherResponse: 'static,
        RequestToSelf,
        RequestToSelfResponse,
        Error: 'static,
    >
    GhostContextEndpoint<
        Context,
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
    >
{
    /// internal new used by `GhostEndpoint::as_context_endpoint`
    fn new(
        request_id_prefix: &str,
        sender: crossbeam_channel::Sender<
            GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
        >,
        receiver: crossbeam_channel::Receiver<
            GhostEndpointMessage<RequestToSelf, RequestToOtherResponse, Error>,
        >,
    ) -> Self {
        Self {
            sender,
            receiver,
            pending_responses_tracker: GhostTracker::new(request_id_prefix),
            outbox_messages_to_self: Vec::new(),
        }
    }

    /// publish an event to the remote side, not expecting a response
    pub fn publish(&mut self, payload: RequestToOther) {
        self.sender
            .send(GhostEndpointMessage::Request {
                request_id: None,
                payload,
            })
            .expect("should send");
    }

    /// make a request of the other side. When a response is sent back to us
    /// the callback will be invoked.
    pub fn request<A: Any>(
        &mut self,
        timeout: std::time::Duration,
        context: Context,
        payload: RequestToOther,
        cb: GhostCallback<A, Context, RequestToOtherResponse, Error>,
    ) {
        let request_id = self
            .pending_responses_tracker
            .bookmark(timeout, context, cb);
        self.sender
            .send(GhostEndpointMessage::Request {
                request_id: Some(request_id),
                payload,
            })
            .expect("should send");
    }

    /// fetch any messages (requests or events) sent to us from the other side
    pub fn drain_messages(
        &mut self,
    ) -> Vec<GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>> {
        self.outbox_messages_to_self.drain(..).collect()
    }

    /// check for pending responses timeouts or incoming messages
    pub fn process(&mut self, actor: &mut dyn Any) -> GhostResult<()> {
        self.pending_responses_tracker.process(actor)?;
        loop {
            let msg: Result<
                GhostEndpointMessage<RequestToSelf, RequestToOtherResponse, Error>,
                crossbeam_channel::TryRecvError,
            > = self.receiver.try_recv();
            match msg {
                Ok(channel_message) => match channel_message {
                    GhostEndpointMessage::Request {
                        request_id,
                        payload,
                    } => {
                        self.outbox_messages_to_self.push(GhostMessage::new(
                            request_id,
                            payload,
                            self.sender.clone(),
                        ));
                    }
                    GhostEndpointMessage::Response {
                        request_id,
                        payload,
                    } => {
                        self.pending_responses_tracker
                            .handle(request_id, actor, payload)?;
                    }
                },
                Err(e) => match e {
                    crossbeam_channel::TryRecvError::Empty => {
                        break;
                    }
                    crossbeam_channel::TryRecvError::Disconnected => {
                        return Err("disconnected GhostActor Endpoint".into());
                    }
                },
            }
        }
        Ok(())
    }
}

/// We want to create a two-way communication channel between a GhostActor
/// and its parent. `create_ghost_channel` will output two GhostEndpoint
/// structures, the first one is the parent side, the second is the child's.
#[allow(clippy::complexity)]
pub fn create_ghost_channel<
    RequestToParent,
    RequestToParentResponse: 'static,
    RequestToChild,
    RequestToChildResponse: 'static,
    Error: 'static,
>() -> (
    GhostEndpoint<
        RequestToChild,
        RequestToChildResponse,
        RequestToParent,
        RequestToParentResponse,
        Error,
    >,
    GhostEndpoint<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
    >,
) {
    let (child_send, parent_recv) = crossbeam_channel::unbounded::<
        GhostEndpointMessage<RequestToParent, RequestToChildResponse, Error>,
    >();
    let (parent_send, child_recv) = crossbeam_channel::unbounded::<
        GhostEndpointMessage<RequestToChild, RequestToParentResponse, Error>,
    >();
    let parent_side = GhostEndpoint::new(parent_send, parent_recv);
    let child_side = GhostEndpoint::new(child_send, child_recv);

    (parent_side, child_side)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_ghost_channel_message_event() {
        let (child_send, child_as_parent_recv) = crossbeam_channel::unbounded::<
            GhostEndpointMessage<TestMsgOut, TestMsgInResponse, TestError>,
        >();

        let mut msg: GhostMessage<TestMsgIn, TestMsgOut, TestMsgInResponse, TestError> =
            GhostMessage::new_event(
                TestMsgIn("this is an event message from an internal child".into()),
                child_send,
            );
        assert_eq!("GhostMessage {request_id: None, ..}", format!("{:?}", msg));
        let payload = msg.take_message().unwrap();
        assert_eq!(
            "TestMsgIn(\"this is an event message from an internal child\")",
            format!("{:?}", payload)
        );

        msg.respond(Ok(TestMsgInResponse(
            "response back to child which should fail because no request id".into(),
        )));
        // check to see if the message was sent
        let response = child_as_parent_recv.recv();
        assert_eq!("Err(RecvError)", format!("{:?}", response));
    }

    #[test]
    fn test_ghost_channel_message_request() {
        let (child_send, child_as_parent_recv) = crossbeam_channel::unbounded::<
            GhostEndpointMessage<TestMsgOut, TestMsgInResponse, TestError>,
        >();
        let request_id = RequestId::new();
        let msg: GhostMessage<TestMsgIn, TestMsgOut, TestMsgInResponse, TestError> =
            GhostMessage::new_request(
                request_id.clone(),
                TestMsgIn("this is a request message from an internal child".into()),
                child_send,
            );
        msg.respond(Ok(TestMsgInResponse("response back to child".into())));

        // check to see if the response was sent
        let response = child_as_parent_recv.recv();
        match response {
            Ok(GhostEndpointMessage::Response {
                request_id: req_id,
                payload,
            }) => {
                assert_eq!(req_id, request_id);
                assert_eq!(
                    "Ok(TestMsgInResponse(\"response back to child\"))",
                    format!("{:?}", payload)
                );
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_ghost_channel_endpoint() {
        #[derive(Debug)]
        struct FakeActor(String);

        // this genrates a callback for requests that simply puts the callbackdata  into
        // the FakeActor's state String, thus for testing we can just look in the actors's
        // state to see if the callback was run.
        fn cb_factory() -> GhostCallback<FakeActor, TestContext, TestMsgOutResponse, TestError> {
            Box::new(|me, _context, callback_data| {
                me.0 = format!("{:?}", callback_data);
                Ok(())
            })
        }

        let fake_dyn_actor = &mut FakeActor("".to_string());

        // build the channel which returns two endpoints with cross-connected crossbeam channels
        let (parent_side, child_side) = create_ghost_channel::<
            TestMsgOut,
            TestMsgOutResponse,
            TestMsgIn,
            TestMsgInResponse,
            TestError,
        >();

        // in this test the endpoint will be the child end
        let mut endpoint = child_side.as_context_endpoint("req_id_prefix");

        endpoint.publish(TestMsgOut("event to my parent".into()));
        // check to see if the event was sent to the parent
        let msg = parent_side.receiver.recv();
        match msg {
            Ok(GhostEndpointMessage::Request {
                request_id,
                payload,
            }) => {
                assert_eq!(request_id, None);
                assert_eq!(
                    "TestMsgOut(\"event to my parent\")",
                    format!("{:?}", payload)
                );
            }
            _ => assert!(false),
        }

        endpoint.request(
            std::time::Duration::from_millis(1000),
            TestContext("context data".into()),
            TestMsgOut("request to my parent".into()),
            cb_factory(),
        );
        // simulate receiving this on the parent-side and check that the
        // correct message went into the channel
        let msg = parent_side.receiver.recv();
        match msg {
            Ok(GhostEndpointMessage::Request {
                request_id,
                payload,
            }) => {
                assert!(request_id.is_some());
                assert_eq!(
                    "TestMsgOut(\"request to my parent\")",
                    format!("{:?}", payload)
                );

                // and simulate sending a response from the parent side
                parent_side
                    .sender
                    .send(GhostEndpointMessage::Response {
                        request_id: request_id.unwrap(),
                        payload: Ok(TestMsgOutResponse("response from parent".into())),
                    })
                    .expect("should send");
            }
            _ => assert!(false),
        }

        assert_eq!("", fake_dyn_actor.0);
        assert!(endpoint.process(fake_dyn_actor).is_ok());
        assert_eq!(
            "Response(Ok(TestMsgOutResponse(\"response from parent\")))",
            fake_dyn_actor.0
        );

        // Now we'll send a request that should timeout
        endpoint.request(
            std::time::Duration::from_millis(1),
            TestContext("context data".into()),
            TestMsgOut("another request to my parent".into()),
            cb_factory(),
        );

        // wait 1 ms for the callback to have expired
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(endpoint.process(fake_dyn_actor).is_ok());
        assert_eq!("Timeout", fake_dyn_actor.0);

        // now lets simulate sending an event from the parent
        parent_side
            .sender
            .send(GhostEndpointMessage::Request {
                request_id: None,
                payload: TestMsgIn("event from a parent".into()),
            })
            .expect("should send");

        assert_eq!(endpoint.drain_messages().len(), 0);
        // calling process should then cause this message to be added the endpoint's inbox
        // which we get access to by calling drain_messages()
        assert!(endpoint.process(fake_dyn_actor).is_ok());
        let mut messages = endpoint.drain_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(
            "Some(TestMsgIn(\"event from a parent\"))",
            format!("{:?}", messages[0].take_message())
        );
    }
}
