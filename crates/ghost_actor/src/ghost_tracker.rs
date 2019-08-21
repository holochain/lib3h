use std::collections::HashMap;

use crate::RequestId;

pub enum GhostCallbackData<CbData> {
    Response(CbData),
    Timeout,
}

pub type GhostCallback<'cb, GA, CbData, E> =
    Box<dyn Fn(&mut GA, GhostCallbackData<CbData>) -> Result<(), E> + 'cb>;

struct GhostTrackerEntry<'gte, GA, CbData, E> {
    expires: std::time::SystemTime,
    cb: GhostCallback<'gte, GA, CbData, E>,
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

    pub fn bookmark(
        &mut self,
        timeout: std::time::Duration,
        cb: GhostCallback<'gtrack, GA, CbData, E>,
    ) -> RequestId {
        let request_id = RequestId::with_prefix(&self.request_id_prefix);
        self.pending.insert(
            request_id.clone(),
            GhostTrackerEntry {
                expires: std::time::SystemTime::now()
                    .checked_add(timeout)
                    .expect("can add timeout to SystemTime::now()"),
                cb,
            },
        );
        request_id
    }

    pub fn handle(&mut self, request_id: RequestId, ga: &mut GA, data: CbData) -> Result<(), E> {
        match self.pending.remove(&request_id) {
            None => {
                println!("request_id {:?} not found", request_id);
                Ok(())
            }
            Some(entry) => (entry.cb)(ga, GhostCallbackData::Response(data)),
        }
    }
}
