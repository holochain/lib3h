use std::collections::HashMap;

use crate::{DidWork, RequestId};

pub type Callback<'cb, GA, CbData> = Box<dyn Fn(&mut GA, CbData) + 'cb>;

pub struct GhostTracker<'gtrack, GA, CbData, E> {
    request_id_prefix: String,
    pending: HashMap<RequestId, Callback<'gtrack, GA, CbData>>,
    phantom_error: std::marker::PhantomData<E>,
}

impl<'gtrack, GA, CbData, E> GhostTracker<'gtrack, GA, CbData, E> {
    pub fn new(request_id_prefix: &str) -> Self {
        Self {
            request_id_prefix: request_id_prefix.to_string(),
            pending: HashMap::new(),
            phantom_error: std::marker::PhantomData,
        }
    }

    pub fn bookmark(&mut self, cb: Callback<'gtrack, GA, CbData>) -> RequestId {
        let request_id = RequestId::with_prefix(&self.request_id_prefix);
        self.pending.insert(request_id.clone(), cb);
        request_id
    }

    pub fn handle(&mut self, request_id: RequestId, ga: &mut GA, data: CbData) -> Result<(), E> {
        match self.pending.remove(&request_id) {
            None => println!("request_id {:?} not found", request_id),
            Some(cb) => cb(ga, data),
        }
        Ok(())
    }
}

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

    pub fn process(&mut self) -> Result<DidWork, E> {
        Ok(true.into())
    }

    pub fn send_out_request(
        &mut self,
        request: RequestAsChild,
        cb: Box<dyn Fn(&mut GA, ResponseAsChild) + 'gas>,
    ) {
        let request_id = self.callbacks.bookmark(cb);
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
