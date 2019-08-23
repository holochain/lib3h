use crate::{RequestId, GhostTracker, GhostCallback, GhostResult};

pub type FixmeContext = String;

enum GhostChannelMessage<Request, Response, Error> {
    Request { request_id: Option<RequestId>, payload: Request },
    Response { request_id: RequestId, payload: Result<Response, Error> },
}

pub struct GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error> {
    request_id: Option<RequestId>,
    payload: RequestToSelf,
    sender: crossbeam_channel::Sender<GhostChannelMessage<RequestToOther, RequestToSelfResponse, Error>>,
}

impl<RequestToSelf, RequestToOther, RequestToSelfResponse, Error> GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error> {
    fn new(request_id: Option<RequestId>, payload: RequestToSelf, sender: crossbeam_channel::Sender<GhostChannelMessage<RequestToOther, RequestToSelfResponse, Error>>) -> Self {
        Self {
            request_id,
            payload,
            sender,
        }
    }

    pub fn payload(&self) -> &RequestToSelf {
        &self.payload
    }

    pub fn respond(self, payload: Result<RequestToSelfResponse, Error>) {
        if let Some(request_id) = &self.request_id {
            self.sender.send(GhostChannelMessage::Response {
                request_id: request_id.clone(),
                payload,
            }).expect("should send");
        }
    }
}

pub struct GhostChannel<RequestToOther, RequestToOtherResponse, RequestToSelf, RequestToSelfResponse, Error> {
    sender: crossbeam_channel::Sender<GhostChannelMessage<RequestToOther, RequestToSelfResponse, Error>>,
    receiver: crossbeam_channel::Receiver<GhostChannelMessage<RequestToSelf, RequestToOtherResponse, Error>>,
    await_request_to_other_response: GhostTracker<FixmeContext, RequestToOtherResponse, Error>,
    outbox_requests_to_self: Vec<GhostMessage<RequestToSelf, RequestToOther, RequestToSelfResponse, Error>>,
}

impl<RequestToOther, RequestToOtherResponse, RequestToSelf, RequestToSelfResponse, Error> GhostChannel<RequestToOther, RequestToOtherResponse, RequestToSelf, RequestToSelfResponse, Error> {
    fn new(sender: crossbeam_channel::Sender<GhostChannelMessage<RequestToOther, RequestToSelfResponse, Error>>, receiver: crossbeam_channel::Receiver<GhostChannelMessage<RequestToSelf, RequestToOtherResponse, Error>>) -> Self {
        Self {
            sender,
            receiver,
            await_request_to_other_response: GhostTracker::new("uuuh?"),
            outbox_requests_to_self: Vec::new(),
        }
    }

    pub fn publish(&mut self, payload: RequestToOther) {
        self.sender.send(GhostChannelMessage::Request {
            request_id: None,
            payload,
        }).expect("should send");
    }

    pub fn request(&mut self, payload: RequestToOther, cb: GhostCallback<FixmeContext, RequestToOtherResponse, Error>) {
        let request_id = self.await_request_to_other_response.bookmark(
            std::time::Duration::from_millis(2000),
            "fix-me".to_string(),
            cb
        );
        self.sender.send(GhostChannelMessage::Request {
            request_id: Some(request_id),
            payload,
        }).expect("should send");
    }

    pub fn process(&mut self) -> GhostResult<()> {
        loop {
            let msg: Result<
                GhostChannelMessage<
                    RequestToSelf, RequestToOtherResponse, Error
                >,
                crossbeam_channel::TryRecvError,
            > = self.receiver.try_recv();
            match msg {
                Ok(channel_message) => {
                    match channel_message {
                        GhostChannelMessage::Request { request_id, payload } => {
                            self.outbox_requests_to_self.push(GhostMessage::new(request_id, payload, self.sender.clone()));
                        }
                        GhostChannelMessage::Response { request_id, payload } => {
                            self.await_request_to_other_response.handle(
                                request_id,
                                &mut "fixme-any".to_string(),
                                payload,
                            )?;
                        }
                    }
                }
                Err(e) => {
                    match e {
                        crossbeam_channel::TryRecvError::Empty => {
                            break;
                        }
                        crossbeam_channel::TryRecvError::Disconnected => {
                            return Err("disconnected GhostActor Channel".into());
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn create_ghost_channel<RequestToChild, RequestToChildResponse, RequestToParent, RequestToParentResponse, Error>() -> (GhostChannel<RequestToChild, RequestToChildResponse, RequestToParent, RequestToParentResponse, Error>, GhostChannel<RequestToParent, RequestToParentResponse, RequestToChild, RequestToChildResponse, Error>) {
    let (child_send, parent_recv) = crossbeam_channel::unbounded::<GhostChannelMessage<RequestToParent, RequestToChildResponse, Error>>();
    let (parent_send, child_recv) = crossbeam_channel::unbounded::<GhostChannelMessage<RequestToChild, RequestToParentResponse, Error>>();
    let parent_side = GhostChannel::new(parent_send, parent_recv);
    let child_side = GhostChannel::new(child_send, child_recv);

    (parent_side, child_side)
}
