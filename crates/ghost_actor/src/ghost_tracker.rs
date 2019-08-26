use std::{any::Any, collections::HashMap};

use crate::{GhostResult, RequestId};

/// a ghost request callback can be invoked with a response that was injected
/// into the system through the `handle` pathway, or to indicate a failure
/// such as a timeout
#[derive(Debug)]
pub enum GhostCallbackData<CbData, E> {
    Response(Result<CbData, E>),
    Timeout,
}

/// definition for a ghost request callback
/// note, the callback can be registered as `'static` because the code
/// definition itself doesn't depend on any specific instance lifetime
/// if you want to mutate the state of a struct instance, pass it in
/// with the `handle` or `process` call.
/// (see detach crate for help with self refs)
pub type GhostCallback<Context, CbData, E> =
    Box<dyn Fn(&mut dyn Any, Context, GhostCallbackData<CbData, E>) -> GhostResult<()> + 'static>;

/// this internal struct helps us keep track of the context and timeout
/// for a callback that was bookmarked in the tracker
struct GhostTrackerEntry<Context, CbData, E> {
    expires: std::time::SystemTime,
    context: Context,
    cb: GhostCallback<Context, CbData, E>,
}

/// GhostTracker registers callbacks associated with request_ids
/// that can be triggered later when a response comes back indicating that id
pub struct GhostTracker<Context, CbData, E> {
    request_id_prefix: String,
    pending: HashMap<RequestId, GhostTrackerEntry<Context, CbData, E>>,
}

impl<Context, CbData, E> GhostTracker<Context, CbData, E> {
    /// create a new tracker instance (with request_id prefix)
    pub fn new(request_id_prefix: &str) -> Self {
        Self {
            request_id_prefix: request_id_prefix.to_string(),
            pending: HashMap::new(),
        }
    }

    /// trigger any periodic or delayed callbacks
    /// also check / cleanup any timeouts
    pub fn process(&mut self, ga: &mut dyn Any) -> GhostResult<()> {
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
                    (entry.cb)(ga, entry.context, GhostCallbackData::Timeout)?;
                }
            }
        }

        Ok(())
    }

    /// register a callback
    pub fn bookmark(
        &mut self,
        timeout: std::time::Duration,
        context: Context,
        cb: GhostCallback<Context, CbData, E>,
    ) -> RequestId {
        let request_id = RequestId::with_prefix(&self.request_id_prefix);
        self.pending.insert(
            request_id.clone(),
            GhostTrackerEntry {
                expires: std::time::SystemTime::now()
                    .checked_add(timeout)
                    .expect("can add timeout to SystemTime::now()"),
                context,
                cb,
            },
        );
        request_id
    }

    /// handle a response
    pub fn handle(
        &mut self,
        request_id: RequestId,
        ga: &mut dyn Any,
        data: Result<CbData, E>,
    ) -> GhostResult<()> {
        match self.pending.remove(&request_id) {
            None => {
                println!("request_id {:?} not found", request_id);
                Ok(())
            }
            Some(entry) => (entry.cb)(ga, entry.context, GhostCallbackData::Response(data)),
        }
    }
}
