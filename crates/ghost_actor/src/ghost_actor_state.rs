use crate::{DidWork, GhostCallback, GhostTracker, RequestId};
use std::any::Any;

pub struct GhostActorState<RequestToParent, RequestToParentResponse, RequestToChildResponse, E> {
    callbacks: GhostTracker<RequestToParentResponse, E>,
    requests_to_parent: Vec<(Option<RequestId>, RequestToParent)>,
    responses_to_parent: Vec<(RequestId, RequestToChildResponse)>,
    phantom_error: std::marker::PhantomData<E>,
}

impl<RequestToParent, RequestToParentResponse, RequestToChildResponse, E>
    GhostActorState<RequestToParent, RequestToParentResponse, RequestToChildResponse, E>
{
    pub fn new() -> Self {
        Self {
            callbacks: GhostTracker::new("testing"),
            requests_to_parent: Vec::new(),
            responses_to_parent: Vec::new(),
            phantom_error: std::marker::PhantomData,
        }
    }

    pub fn process(&mut self, ga: &mut dyn Any) -> Result<DidWork, E> {
        self.callbacks.process(ga)?;
        Ok(true.into())
    }

    /// called by concrete implementation
    pub fn send_event_to_parent(&mut self, event: RequestToParent) {
        self.requests_to_parent.push((None, event));
    }

    /// called by concrete implementation
    pub fn send_request_to_parent(
        &mut self,
        timeout: std::time::Duration,
        request: RequestToParent,
        cb: GhostCallback<RequestToParentResponse, E>,
    ) {
        let request_id = self.callbacks.bookmark(timeout, cb);
        self.requests_to_parent.push((Some(request_id), request));
    }

    /// this is called by GhostActor when a parent calls `ga.respond()`
    pub(crate) fn handle_response(
        &mut self,
        ga: &mut dyn Any,
        request_id: RequestId,
        response: RequestToParentResponse,
    ) -> Result<(), E> {
        self.callbacks.handle(request_id, ga, response)
    }

    /// our parent sent in a request
    /// we have a response to that request
    /// post it, so they can get the response through `drain_responses()`
    /// if this was a synchronous action, this will be called inside
    /// GhostActor::request()
    pub fn respond_to_parent(&mut self, request_id: RequestId, response: RequestToChildResponse) {
        self.responses_to_parent.push((request_id, response));
    }

    pub fn drain_requests(&mut self) -> Vec<(Option<RequestId>, RequestToParent)> {
        self.requests_to_parent.drain(..).collect()
    }

    pub fn drain_responses(&mut self) -> Vec<(RequestId, RequestToChildResponse)> {
        self.responses_to_parent.drain(..).collect()
    }
}
