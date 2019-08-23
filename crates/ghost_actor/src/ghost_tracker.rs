use std::{any::Any, collections::HashMap};

use crate::{GhostResult, RequestId};

#[derive(Debug)]
pub enum GhostCallbackData<CbData, E> {
    Response(Result<CbData, E>),
    Timeout,
}

pub type GhostCallback<Context, CbData, E> =
    Box<dyn Fn(&mut dyn Any, Context, GhostCallbackData<CbData, E>) -> GhostResult<()> + 'static>;

#[macro_export]
macro_rules! ghost_cb_call {
    ( $cb:expr, $mod:expr, $ctx:expr, $data:expr ) => {{
        let tmp = std::mem::replace(&mut $cb, Box::new(|_, _, _|{}));
        let out = tmp($mod, $ctx, $data);
        std::mem::replace(&mut $cb, tmp);
        out
    }}
}

struct GhostTrackerEntry<Context, CbData, E> {
    expires: std::time::SystemTime,
    context: Context,
    cb: GhostCallback<Context, CbData, E>,
}

pub struct GhostTracker<Context, CbData, E> {
    request_id_prefix: String,
    pending: HashMap<RequestId, GhostTrackerEntry<Context, CbData, E>>,
    phantom_error: std::marker::PhantomData<E>,
}

impl<Context, CbData, E> GhostTracker<Context, CbData, E> {
    pub fn new(request_id_prefix: &str) -> Self {
        Self {
            request_id_prefix: request_id_prefix.to_string(),
            pending: HashMap::new(),
            phantom_error: std::marker::PhantomData,
        }
    }

    /// called by ActorState::process(), or GhostActors needing tracking
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
