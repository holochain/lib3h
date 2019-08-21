use std::collections::HashMap;

use crate::{DidWork, RequestId};

pub type Callback<'cb, GA, CB> = Box<dyn Fn(&mut GA, CB) + 'cb>;

pub struct GhostTracker<'gtrack, GA, FromChild, ToChild, ToParent, E> {
    expected_to_child: HashMap<RequestId, Callback<'gtrack, GA, ToChild>>,
    #[allow(dead_code)]
    expected_to_parent: HashMap<RequestId, Callback<'gtrack, GA, ToParent>>,
    out_outbox: Vec<(Option<RequestId>, FromChild)>,
    in_outbox: Vec<(RequestId, ToParent)>,
    phantom_error: std::marker::PhantomData<E>,
}

impl<'gtrack, GA, FromChild, ToChild, ToParent, E>
    GhostTracker<'gtrack, GA, FromChild, ToChild, ToParent, E>
{
    pub fn new() -> Self {
        Self {
            expected_to_child: HashMap::new(),
            expected_to_parent: HashMap::new(),
            out_outbox: Vec::new(),
            in_outbox: Vec::new(),
            phantom_error: std::marker::PhantomData,
        }
    }

    pub fn process(&mut self) -> Result<DidWork, E> {
        Ok(true.into())
    }

    pub fn send_out_request(
        &mut self,
        request: FromChild,
        cb: Box<dyn Fn(&mut GA, ToChild) + 'gtrack>,
    ) {
        let request_id = RequestId("".to_string());
        self.expected_to_child.insert(request_id.clone(), cb);
        self.out_outbox.push((Some(request_id), request));
    }

    pub fn handle_out_response(&mut self, m: &mut GA, request_id: RequestId, response: ToChild) {
        match self.expected_to_child.remove(&request_id) {
            None => println!("no pending for request_id {}", request_id.0),
            Some(cb) => cb(m, response),
        }
    }

    pub fn post_in_response(&mut self, request_id: RequestId, response: ToParent) {
        self.in_outbox.push((request_id, response));
    }

    pub fn drain_requests(&mut self) -> Vec<(Option<RequestId>, FromChild)> {
        self.out_outbox.drain(..).collect()
    }

    pub fn drain_responses(&mut self) -> Vec<(RequestId, ToParent)> {
        self.in_outbox.drain(..).collect()
    }
}
