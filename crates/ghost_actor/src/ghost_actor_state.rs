use crate::{DidWork, RequestId, GhostTracker, GhostCallback};

pub struct GhostActorState<'gas, GA, RequestAsChild, ResponseAsChild, ResponseToParent, E> {
    callbacks: GhostTracker<'gas, GA, ResponseAsChild, E>,
    requests_to_parent: Vec<(Option<RequestId>, RequestAsChild)>,
    responses_to_parent: Vec<(RequestId, ResponseToParent)>,
    phantom_error: std::marker::PhantomData<E>,
}

impl<'gas, GA, RequestAsChild, ResponseAsChild, ResponseToParent, E>
    GhostActorState<'gas, GA, RequestAsChild, ResponseAsChild, ResponseToParent, E>
{
    pub fn new() -> Self {
        Self {
            callbacks: GhostTracker::new("testing"),
            requests_to_parent: Vec::new(),
            responses_to_parent: Vec::new(),
            phantom_error: std::marker::PhantomData,
        }
    }

    pub fn process(&mut self, ga: &mut GA) -> Result<DidWork, E> {
        self.callbacks.process(ga)?;
        Ok(true.into())
    }

    /// called by concrete implementation
    pub fn send_event_to_parent(
        &mut self,
        event: RequestAsChild,
    ) {
        self.requests_to_parent.push((None, event));
    }

    /// called by concrete implementation
    pub fn send_request_to_parent(
        &mut self,
        timeout: std::time::Duration,
        request: RequestAsChild,
        cb: GhostCallback<'gas, GA, ResponseAsChild, E>,
    ) {
        let request_id = self.callbacks.bookmark(timeout, cb);
        self.requests_to_parent.push((Some(request_id), request));
    }

    /// this is called by GhostActor when a parent calls `ga.respond()`
    pub(crate) fn handle_response(
        &mut self,
        ga: &mut GA,
        request_id: RequestId,
        response: ResponseAsChild,
    ) -> Result<(), E> {
        self.callbacks.handle(request_id, ga, response)
    }

    pub fn post_in_response(&mut self, request_id: RequestId, response: ResponseToParent) {
        self.responses_to_parent.push((request_id, response));
    }

    pub fn drain_requests(&mut self) -> Vec<(Option<RequestId>, RequestAsChild)> {
        self.requests_to_parent.drain(..).collect()
    }

    pub fn drain_responses(&mut self) -> Vec<(RequestId, ResponseToParent)> {
        self.responses_to_parent.drain(..).collect()
    }
}
