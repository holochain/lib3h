use std::collections::HashMap;

use crate::{DidWork, RequestId};

pub enum GhostCallbackData<CbData> {
    Response(CbData),
    Timeout,
}

pub type Callback<'cb, GA, CbData, E> = Box<dyn Fn(&mut GA, GhostCallbackData<CbData>) -> Result<(), E> + 'cb>;

struct GhostTrackerEntry<'gte, GA, CbData, E> {
    expires: std::time::SystemTime,
    cb: Callback<'gte, GA, CbData, E>,
}

pub struct GhostTracker<'gtrack, GA, CbData, E> {
    request_id_prefix: String,
    pending: HashMap<RequestId, GhostTrackerEntry<'gtrack, GA, CbData, E>>,
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

    /// called by ActorState::process(), or GhostActors needing tracking
    pub fn process(&mut self, ga: &mut GA) -> Result<(), E> {
        let mut expired = Vec::new();

        let now = std::time::SystemTime::now();

        for (request_id, entry) in self.pending.iter() {
            if now > entry.expires {
                expired.push(request_id.clone())
            }
        }

        for request_id in expired {
            match self.pending.remove(&request_id) {
                None => (),
                Some(entry) => {
                    (entry.cb)(ga, GhostCallbackData::Timeout)?;
                }
            }
        }

        Ok(())
    }

    pub fn bookmark(&mut self, timeout: std::time::Duration, cb: Callback<'gtrack, GA, CbData, E>) -> RequestId {
        let request_id = RequestId::with_prefix(&self.request_id_prefix);
        self.pending.insert(request_id.clone(), GhostTrackerEntry {
            expires: std::time::SystemTime::now().checked_add(timeout).expect("can add timeout to SystemTime::now()"),
            cb
        });
        request_id
    }

    pub fn handle(&mut self, request_id: RequestId, ga: &mut GA, data: CbData) -> Result<(), E> {
        match self.pending.remove(&request_id) {
            None => {
                println!("request_id {:?} not found", request_id);
                Ok(())
            }
            Some(entry) => {
                (entry.cb)(ga, GhostCallbackData::Response(data))
            }
        }
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
        cb: Callback<'gas, GA, ResponseAsChild, E>,
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
