use crate::{GhostCallback, GhostResult, GhostTracker, RequestId};
use std::any::Any;

enum GhostChannelMessage<Request, Response, Error> {
    Request {
        request_id: Option<RequestId>,
        payload: Request,
    },
    Response {
        request_id: RequestId,
        payload: Result<Response, Error>,
    },
}

pub struct GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error> {
    request_id: Option<RequestId>,
    payload: Option<RequestToSelf>,
    sender: crossbeam_channel::Sender<
        GhostChannelMessage<RequestToOther, RequestToSelfResponse, Error>,
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
        payload: RequestToSelf,
        sender: crossbeam_channel::Sender<
            GhostChannelMessage<RequestToOther, RequestToSelfResponse, Error>,
        >,
    ) -> Self {
        Self {
            request_id,
            payload: Some(payload),
            sender,
        }
    }

    pub fn take_payload(&mut self) -> Option<RequestToSelf> {
        std::mem::replace(&mut self.payload, None)
    }

    pub fn respond(self, payload: Result<RequestToSelfResponse, Error>) {
        if let Some(request_id) = &self.request_id {
            self.sender
                .send(GhostChannelMessage::Response {
                    request_id: request_id.clone(),
                    payload,
                })
                .expect("should send");
        }
    }
}

pub struct GhostChannel<
    RequestToOther,
    RequestToOtherResponse,
    RequestToSelf,
    RequestToSelfResponse,
    Error,
> {
    sender: crossbeam_channel::Sender<
        GhostChannelMessage<RequestToOther, RequestToSelfResponse, Error>,
    >,
    receiver: crossbeam_channel::Receiver<
        GhostChannelMessage<RequestToSelf, RequestToOtherResponse, Error>,
    >,
}

impl<RequestToOther, RequestToOtherResponse, RequestToSelf, RequestToSelfResponse, Error>
    GhostChannel<
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
    >
{
    fn new(
        sender: crossbeam_channel::Sender<
            GhostChannelMessage<RequestToOther, RequestToSelfResponse, Error>,
        >,
        receiver: crossbeam_channel::Receiver<
            GhostChannelMessage<RequestToSelf, RequestToOtherResponse, Error>,
        >,
    ) -> Self {
        Self { sender, receiver }
    }

    pub fn as_context_channel<Context>(
        self,
    ) -> GhostContextChannel<
        Context,
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
    > {
        GhostContextChannel::new(self.sender, self.receiver)
    }
}

pub struct GhostContextChannel<
    Context,
    RequestToOther,
    RequestToOtherResponse,
    RequestToSelf,
    RequestToSelfResponse,
    Error,
> {
    sender: crossbeam_channel::Sender<
        GhostChannelMessage<RequestToOther, RequestToSelfResponse, Error>,
    >,
    receiver: crossbeam_channel::Receiver<
        GhostChannelMessage<RequestToSelf, RequestToOtherResponse, Error>,
    >,
    await_request_to_other_response: GhostTracker<Context, RequestToOtherResponse, Error>,
    outbox_requests_to_self:
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
    GhostContextChannel<
        Context,
        RequestToOther,
        RequestToOtherResponse,
        RequestToSelf,
        RequestToSelfResponse,
        Error,
    >
{
    fn new(
        sender: crossbeam_channel::Sender<
            GhostChannelMessage<RequestToOther, RequestToSelfResponse, Error>,
        >,
        receiver: crossbeam_channel::Receiver<
            GhostChannelMessage<RequestToSelf, RequestToOtherResponse, Error>,
        >,
    ) -> Self {
        Self {
            sender,
            receiver,
            await_request_to_other_response: GhostTracker::new("uuuh?"),
            outbox_requests_to_self: Vec::new(),
        }
    }

    pub fn publish(&mut self, payload: RequestToOther) {
        self.sender
            .send(GhostChannelMessage::Request {
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
            .await_request_to_other_response
            .bookmark(timeout, context, cb);
        self.sender
            .send(GhostChannelMessage::Request {
                request_id: Some(request_id),
                payload,
            })
            .expect("should send");
    }

    pub fn drain_requests(
        &mut self,
    ) -> Vec<GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>> {
        self.outbox_requests_to_self.drain(..).collect()
    }

    pub fn process(&mut self, actor: &mut dyn Any) -> GhostResult<()> {
        loop {
            let msg: Result<
                GhostChannelMessage<RequestToSelf, RequestToOtherResponse, Error>,
                crossbeam_channel::TryRecvError,
            > = self.receiver.try_recv();
            match msg {
                Ok(channel_message) => match channel_message {
                    GhostChannelMessage::Request {
                        request_id,
                        payload,
                    } => {
                        self.outbox_requests_to_self.push(GhostMessage::new(
                            request_id,
                            payload,
                            self.sender.clone(),
                        ));
                    }
                    GhostChannelMessage::Response {
                        request_id,
                        payload,
                    } => {
                        self.await_request_to_other_response
                            .handle(request_id, actor, payload)?;
                    }
                },
                Err(e) => match e {
                    crossbeam_channel::TryRecvError::Empty => {
                        break;
                    }
                    crossbeam_channel::TryRecvError::Disconnected => {
                        return Err("disconnected GhostActor Channel".into());
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
    GhostChannel<
        RequestToChild,
        RequestToChildResponse,
        RequestToParent,
        RequestToParentResponse,
        Error,
    >,
    GhostChannel<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Error,
    >,
) {
    let (child_send, parent_recv) = crossbeam_channel::unbounded::<
        GhostChannelMessage<RequestToParent, RequestToChildResponse, Error>,
    >();
    let (parent_send, child_recv) = crossbeam_channel::unbounded::<
        GhostChannelMessage<RequestToChild, RequestToParentResponse, Error>,
    >();
    let parent_side = GhostChannel::new(parent_send, parent_recv);
    let child_side = GhostChannel::new(child_send, child_recv);

    (parent_side, child_side)
}