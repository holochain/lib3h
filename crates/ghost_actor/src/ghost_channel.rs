use crate::{GhostCallback, GhostResult, GhostTracker, RequestId};
use std::any::Any;

/// enum used internally as the protocol for our crossbeam_channels
/// allows us to be explicit about which messages are requests or responses.
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

impl<RequestToSelf, RequestToOther, RequestToSelfResponse, Error> std::fmt::Debug
    for GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GhostMessage {{ .. }}")
    }
}

impl<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>
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

impl<RequestToOther, RequestToOtherResponse, RequestToSelf, RequestToSelfResponse, Error>
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
    pub fn as_context_endpoint<Context>(
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
        Context,
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
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
    pub fn request(
        &mut self,
        timeout: std::time::Duration,
        context: Context,
        payload: RequestToOther,
        cb: GhostCallback<Context, RequestToOtherResponse, Error>,
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
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    Error,
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
