use crate::{GhostCallback, GhostResult, GhostTracker, RequestId};
use std::any::Any;

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

    pub fn take_message(&mut self) -> Option<RequestToSelf> {
        std::mem::replace(&mut self.message, None)
    }

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

    pub fn as_context_channel<Context>(
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

    pub fn publish(&mut self, payload: RequestToOther) {
        self.sender
            .send(GhostEndpointMessage::Request {
                request_id: None,
                payload,
            })
            .expect("should send");
    }

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

    pub fn drain_messages(
        &mut self,
    ) -> Vec<GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>> {
        self.outbox_messages_to_self.drain(..).collect()
    }

    pub fn process(&mut self, actor: &mut dyn Any) -> GhostResult<()> {
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
