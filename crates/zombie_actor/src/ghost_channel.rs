use crate::{
    ghost_error::ErrorKind, GhostCallback, GhostError, GhostResult, GhostTracker,
    GhostTrackerBookmarkOptions, GhostTrackerBuilder, RequestId, WorkWasDone,
};
use holochain_tracing::Span;

/// enum used internally as the protocol for our crossbeam_channels
/// allows us to be explicit about which messages are requests or responses.
#[derive(Debug)]
enum GhostEndpointMessage<Request: 'static, Response: 'static, Error: 'static> {
    Request {
        requester_bt: backtrace::Backtrace,
        request_id: Option<RequestId>,
        payload: Request,
        span: Span,
    },
    Response {
        responder_bt: backtrace::Backtrace,
        request_id: RequestId,
        payload: Result<Response, Error>,
        span: Span,
    },
}

/// GhostContextEndpoints allow you to drain these incoming `GhostMessage`s
/// A GhostMessage contains the incoming request, as well as a hook to
/// allow a response to automatically be returned.
pub struct GhostMessage<
    MessageToSelf: 'static,
    MessageToOther: 'static,
    MessageToSelfResponse: 'static,
    Error: 'static,
> {
    requester_bt: backtrace::Backtrace,
    request_id: Option<RequestId>,
    message: Option<MessageToSelf>,
    sender: crossbeam_channel::Sender<
        GhostEndpointMessage<MessageToOther, MessageToSelfResponse, Error>,
    >,
    span: Span,
}

impl<
        RequestToSelf: 'static,
        RequestToOther: 'static,
        RequestToSelfResponse: 'static,
        Error: 'static,
    > std::fmt::Debug
    for GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GhostMessage {{request_id: {:?}, ..}}", self.request_id)
    }
}

impl<
        RequestToSelf: 'static,
        RequestToOther: 'static,
        RequestToSelfResponse: 'static,
        Error: 'static + std::fmt::Debug,
    > GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>
{
    fn new(
        requester_bt: backtrace::Backtrace,
        request_id: Option<RequestId>,
        message: RequestToSelf,
        sender: crossbeam_channel::Sender<
            GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
        >,
        span: Span,
    ) -> Self {
        Self {
            requester_bt,
            request_id,
            message: Some(message),
            sender,
            span,
        }
    }

    //#[cfg(test)]
    pub fn test_constructor() -> (Self) {
        let (sender, _receiver) = crossbeam_channel::unbounded();
        Self {
            requester_bt: backtrace::Backtrace::new(),
            request_id: None,
            message: None,
            sender,
            span: Span::fixme(),
        }
    }

    /// create a request message
    #[allow(dead_code)]
    fn new_request(
        requester_bt: backtrace::Backtrace,
        request_id: RequestId,
        message: RequestToSelf,
        sender: crossbeam_channel::Sender<
            GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
        >,
        span: Span,
    ) -> Self {
        GhostMessage::new(requester_bt, Some(request_id), message, sender, span)
    }

    /// create an event message
    #[allow(dead_code)]
    fn new_event(
        requester_bt: backtrace::Backtrace,
        message: RequestToSelf,
        sender: crossbeam_channel::Sender<
            GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
        >,
        span: Span,
    ) -> Self {
        GhostMessage::new(requester_bt, None, message, sender, span)
    }

    /// most often you will want to consume the contents of the request
    /// using take prevents a clone
    pub fn take_message(&mut self) -> Option<RequestToSelf> {
        std::mem::replace(&mut self.message, None)
    }

    pub fn put_message(&mut self, msg: RequestToSelf) {
        self.message = Some(msg);
    }

    /// send a response back to the origin of this request
    pub fn respond(self, payload: Result<RequestToSelfResponse, Error>) -> GhostResult<()> {
        if let Some(request_id) = &self.request_id {
            self.sender.send(GhostEndpointMessage::Response {
                responder_bt: backtrace::Backtrace::new(),
                request_id: request_id.clone(),
                payload,
                span: self.span,
            })?;
        } else {
            // if the user is sending back an error
            // it could get lost here... we are going to panic
            if let Err(e) = payload {
                panic!(
                    "Unhandled publish error: {:?}. You should convert this to a request. BACKTRACE: {:?}",
                    e,
                    self.requester_bt,
                );
            }
        }
        Ok(())
    }

    pub fn is_request(&self) -> bool {
        self.request_id.is_some()
    }

    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn backtrace(&self) -> &backtrace::Backtrace {
        &self.requester_bt
    }
}

/// `create_ghost_channel` outputs two endpoints,
/// a parent_endpoint, and a child_endpoint
/// these raw endpoints are not very useful on their own. When you get them
/// to the place they will be used, you probably want to call
/// `as_context_endpoint_builder()` on them.
pub struct GhostEndpoint<
    RequestToOther: 'static,
    RequestToOtherResponse: 'static,
    RequestToSelf: 'static,
    RequestToSelfResponse: 'static,
    Error: 'static + std::fmt::Debug,
> {
    sender: crossbeam_channel::Sender<
        GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
    >,
    receiver: crossbeam_channel::Receiver<
        GhostEndpointMessage<RequestToSelf, RequestToOtherResponse, Error>,
    >,
}

impl<
        RequestToOther: 'static,
        RequestToOtherResponse: 'static,
        RequestToSelf: 'static,
        RequestToSelfResponse: 'static,
        Error: 'static + std::fmt::Debug,
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
    /// Feel free to use `as_context_endpoint_builder::<()>("prefix")` if you
    /// don't need any context.
    /// request_id_prefix is a debugging hint... the request_ids generated
    /// for tracking request/response pairs will be prepended with this prefix.
    pub fn as_context_endpoint_builder(
        self,
    ) -> GhostContextEndpointBuilder<
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
    > {
        GhostContextEndpointBuilder {
            sender: self.sender,
            receiver: self.receiver,
            tracker_builder: GhostTrackerBuilder::default(),
        }
    }
}

pub struct GhostContextEndpointBuilder<
    RequestToOther: 'static,
    RequestToOtherResponse: 'static,
    RequestToSelf: 'static,
    RequestToSelfResponse: 'static,
    Error: 'static + std::fmt::Debug,
> {
    sender: crossbeam_channel::Sender<
        GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
    >,
    receiver: crossbeam_channel::Receiver<
        GhostEndpointMessage<RequestToSelf, RequestToOtherResponse, Error>,
    >,
    tracker_builder: GhostTrackerBuilder,
}

impl<
        RequestToOther: 'static,
        RequestToOtherResponse: 'static,
        RequestToSelf: 'static,
        RequestToSelfResponse: 'static,
        Error: 'static + std::fmt::Debug,
    >
    GhostContextEndpointBuilder<
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
    >
{
    pub fn build<UserData>(
        self,
    ) -> GhostContextEndpoint<
        UserData,
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
    > {
        GhostContextEndpoint {
            sender: self.sender,
            receiver: self.receiver,
            pending_responses_tracker: self.tracker_builder.build(),
            outbox_messages_to_self: Vec::new(),
        }
    }

    pub fn request_id_prefix(mut self, request_id_prefix: &str) -> Self {
        self.tracker_builder = self.tracker_builder.request_id_prefix(request_id_prefix);
        self
    }

    pub fn default_timeout(mut self, default_timeout: std::time::Duration) -> Self {
        self.tracker_builder = self.tracker_builder.default_timeout(default_timeout);
        self
    }
}

#[derive(Debug, Clone)]
pub struct GhostTrackRequestOptions {
    pub timeout: Option<std::time::Duration>,
}

impl Default for GhostTrackRequestOptions {
    fn default() -> Self {
        Self { timeout: None }
    }
}

impl GhostTrackRequestOptions {
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// indicates this type is able to make callback requests && respond to requests
pub trait GhostCanTrack<
    UserData,
    RequestToOther: 'static,
    RequestToOtherResponse: 'static,
    RequestToSelf: 'static,
    RequestToSelfResponse: 'static,
    Error: 'static + std::fmt::Debug,
>
{
    /// publish an event to the remote side, not expecting a response
    fn publish(&mut self, span: Span, payload: RequestToOther) -> GhostResult<()>;

    /// make a request of the other side. When a response is sent back to us
    /// the callback will be invoked.
    fn request(
        &mut self,
        span: Span,
        payload: RequestToOther,
        cb: GhostCallback<UserData, RequestToOtherResponse, Error>,
    ) -> GhostResult<()>;

    /// make a request of the other side. When a response is sent back to us
    /// the callback will be invoked, override the default timeout.
    fn request_options(
        &mut self,
        span: Span,
        payload: RequestToOther,
        cb: GhostCallback<UserData, RequestToOtherResponse, Error>,
        options: GhostTrackRequestOptions,
    ) -> GhostResult<()>;

    /// fetch any messages (requests or events) sent to us from the other side
    fn drain_messages(
        &mut self,
    ) -> Vec<GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>>;

    /// check for pending responses timeouts or incoming messages
    fn process(&mut self, user_data: &mut UserData) -> GhostResult<WorkWasDone>;
}

/// an expanded endpoint usable to send/receive requests/responses/events
/// see `GhostEndpoint::as_context_endpoint_builder` for additional details
pub struct GhostContextEndpoint<
    UserData,
    RequestToOther: 'static,
    RequestToOtherResponse: 'static,
    RequestToSelf: 'static,
    RequestToSelfResponse: 'static,
    Error: 'static + std::fmt::Debug,
> {
    sender: crossbeam_channel::Sender<
        GhostEndpointMessage<RequestToOther, RequestToSelfResponse, Error>,
    >,
    receiver: crossbeam_channel::Receiver<
        GhostEndpointMessage<RequestToSelf, RequestToOtherResponse, Error>,
    >,
    pending_responses_tracker: GhostTracker<UserData, RequestToOtherResponse, Error>,
    outbox_messages_to_self:
        Vec<GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>>,
}

impl<
        UserData,
        RequestToOther: 'static,
        RequestToOtherResponse: 'static,
        RequestToSelf: 'static,
        RequestToSelfResponse: 'static,
        Error: 'static + std::fmt::Debug,
    >
    GhostContextEndpoint<
        UserData,
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
    >
{
    fn priv_request(
        &mut self,
        mut span: Span,
        payload: RequestToOther,
        cb: GhostCallback<UserData, RequestToOtherResponse, Error>,
        options: GhostTrackRequestOptions,
    ) -> GhostResult<()> {
        let span_bookmark = span.child("bookmark");
        let request_id = match options.timeout {
            None => self.pending_responses_tracker.bookmark(span_bookmark, cb),
            Some(timeout) => self.pending_responses_tracker.bookmark_options(
                span_bookmark,
                cb,
                GhostTrackerBookmarkOptions::default().timeout(timeout),
            ),
        };
        // trace!("ghost_channel: send request {:?}", request_id);
        span.event(format!("ghost_channel: send request {:?}", request_id));
        self.sender.send(GhostEndpointMessage::Request {
            requester_bt: backtrace::Backtrace::new(),
            request_id: Some(request_id),
            payload,
            span: span.follower("send request"),
            // span: span.child("request", |o| o.start()).into(),
        })?;
        Ok(())
    }
}

impl<
        UserData,
        RequestToOther: 'static,
        RequestToOtherResponse: 'static,
        RequestToSelf: 'static,
        RequestToSelfResponse: 'static,
        Error: 'static + std::fmt::Debug,
    >
    GhostCanTrack<
        UserData,
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
    >
    for GhostContextEndpoint<
        UserData,
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
    >
{
    /// publish an event to the remote side, not expecting a response
    fn publish(&mut self, mut span: Span, payload: RequestToOther) -> GhostResult<()> {
        span.event("GhostChannel::publish");
        self.sender.send(GhostEndpointMessage::Request {
            requester_bt: backtrace::Backtrace::new(),
            request_id: None,
            payload,
            span,
        })?;
        Ok(())
    }

    /// make a request of the other side. When a response is sent back to us
    /// the callback will be invoked.
    fn request(
        &mut self,
        mut span: Span,
        payload: RequestToOther,
        cb: GhostCallback<UserData, RequestToOtherResponse, Error>,
    ) -> GhostResult<()> {
        span.event("GhostChannel::request");
        self.priv_request(
            span.child("priv_request"),
            payload,
            cb,
            GhostTrackRequestOptions::default(),
        )
    }

    /// make a request of the other side. When a response is sent back to us
    /// the callback will be invoked, override the default timeout.
    fn request_options(
        &mut self,
        mut span: Span,
        payload: RequestToOther,
        cb: GhostCallback<UserData, RequestToOtherResponse, Error>,
        options: GhostTrackRequestOptions,
    ) -> GhostResult<()> {
        span.event("GhostChannel::request_options");
        self.priv_request(span.child("priv_request"), payload, cb, options)
    }

    /// fetch any messages (requests or events) sent to us from the other side
    fn drain_messages(
        &mut self,
    ) -> Vec<GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>> {
        self.outbox_messages_to_self.drain(..).collect()
    }

    /// check for pending responses timeouts or incoming messages
    fn process(&mut self, user_data: &mut UserData) -> GhostResult<WorkWasDone> {
        let mut work_was_done = self.pending_responses_tracker.process(user_data)?;
        loop {
            let msg: Result<
                GhostEndpointMessage<RequestToSelf, RequestToOtherResponse, Error>,
                crossbeam_channel::TryRecvError,
            > = self.receiver.try_recv();
            match msg {
                Ok(channel_message) => {
                    match channel_message {
                        GhostEndpointMessage::Request {
                            requester_bt,
                            request_id,
                            payload,
                            span,
                        } => {
                            self.outbox_messages_to_self.push(GhostMessage::new(
                                requester_bt,
                                request_id,
                                payload,
                                self.sender.clone(),
                                span.child("outbox_messages"),
                            ));
                        }
                        GhostEndpointMessage::Response {
                            responder_bt: _,
                            request_id,
                            payload,
                            span: _,
                        } => {
                            self.pending_responses_tracker
                                .handle(request_id, user_data, payload)?;
                        }
                    };
                    work_was_done = true.into();
                }
                Err(e) => match e {
                    crossbeam_channel::TryRecvError::Empty => {
                        break;
                    }
                    crossbeam_channel::TryRecvError::Disconnected => {
                        return Err(GhostError::new(ErrorKind::EndpointDisconnected));
                    }
                },
            }
        }
        Ok(work_was_done)
    }
}

/// We want to create a two-way communication channel between a GhostActor
/// and its parent. `create_ghost_channel` will output two GhostEndpoint
/// structures, the first one is the parent side, the second is the child's.
#[allow(clippy::complexity)]
pub fn create_ghost_channel<
    RequestToParent: 'static,
    RequestToParentResponse: 'static,
    RequestToChild: 'static,
    RequestToChildResponse: 'static,
    Error: 'static + std::fmt::Debug,
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
    use holochain_tracing::test_span;
    type TestError = String;

    #[derive(Debug)]
    struct TestMsgOut(String);
    #[derive(Debug)]
    struct TestMsgOutResponse(String);
    #[derive(Debug)]
    struct TestMsgIn(String);
    #[derive(Debug)]
    struct TestMsgInResponse(String);

    #[test]
    fn test_ghost_channel_message_event() {
        let (child_send, child_as_parent_recv) = crossbeam_channel::unbounded::<
            GhostEndpointMessage<TestMsgOut, TestMsgInResponse, TestError>,
        >();

        let mut msg: GhostMessage<TestMsgIn, TestMsgOut, TestMsgInResponse, TestError> =
            GhostMessage::new_event(
                backtrace::Backtrace::new(),
                TestMsgIn("this is an event message from an internal child".into()),
                child_send,
                test_span(""),
            );
        assert_eq!("GhostMessage {request_id: None, ..}", format!("{:?}", msg));
        let payload = msg.take_message().unwrap();
        assert_eq!(
            "TestMsgIn(\"this is an event message from an internal child\")",
            format!("{:?}", payload)
        );

        msg.respond(Ok(TestMsgInResponse(
            "response back to child which should fail because no request id".into(),
        )))
        .unwrap();
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
                backtrace::Backtrace::new(),
                request_id.clone(),
                TestMsgIn("this is a request message from an internal child".into()),
                child_send,
                test_span(""),
            );
        msg.respond(Ok(TestMsgInResponse("response back to child".into())))
            .unwrap();

        // check to see if the response was sent
        let response = child_as_parent_recv.recv();
        match response {
            Ok(GhostEndpointMessage::Response {
                responder_bt: _,
                request_id: req_id,
                payload,
                span: _,
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
        fn cb_factory() -> GhostCallback<FakeActor, TestMsgOutResponse, TestError> {
            Box::new(|me, callback_data| {
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
        let mut endpoint = child_side.as_context_endpoint_builder().build();

        endpoint
            .publish(
                test_span("context data"),
                TestMsgOut("event to my parent".into()),
            )
            .unwrap();
        // check to see if the event was sent to the parent
        let msg = parent_side.receiver.recv();
        match msg {
            Ok(GhostEndpointMessage::Request {
                requester_bt: _,
                request_id,
                payload,
                span: _,
            }) => {
                assert_eq!(request_id, None);
                assert_eq!(
                    "TestMsgOut(\"event to my parent\")",
                    format!("{:?}", payload)
                );
            }
            _ => assert!(false),
        }

        endpoint
            .request(
                test_span("context data"),
                TestMsgOut("request to my parent".into()),
                cb_factory(),
            )
            .unwrap();
        // simulate receiving this on the parent-side and check that the
        // correct message went into the channel
        let msg = parent_side.receiver.recv();
        match msg {
            Ok(GhostEndpointMessage::Request {
                requester_bt: _,
                request_id,
                payload,
                span,
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
                        responder_bt: backtrace::Backtrace::new(),
                        request_id: request_id.unwrap(),
                        payload: Ok(TestMsgOutResponse("response from parent".into())),
                        span,
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
        endpoint
            .request_options(
                test_span("context data"),
                TestMsgOut("another request to my parent".into()),
                cb_factory(),
                GhostTrackRequestOptions::default().timeout(std::time::Duration::from_millis(1)),
            )
            .unwrap();

        // wait 1 ms for the callback to have expired
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(endpoint.process(fake_dyn_actor).is_ok());
        assert_eq!("Timeout", &fake_dyn_actor.0[..7]);

        // now lets simulate sending an event from the parent
        parent_side
            .sender
            .send(GhostEndpointMessage::Request {
                requester_bt: backtrace::Backtrace::new(),
                request_id: None,
                payload: TestMsgIn("event from a parent".into()),
                span: test_span(""),
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
