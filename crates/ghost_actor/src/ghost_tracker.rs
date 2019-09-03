use std::{any::Any, collections::HashMap};

use crate::{ghost_error::ErrorKind, GhostError, GhostResult, RequestId};

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
pub type GhostCallback<A, Context, CbData, E> =
    Box<dyn Fn(&mut A, Context, GhostCallbackData<CbData, E>) -> GhostResult<()> + 'static>;

/// this internal struct helps us keep track of the context and timeout
/// for a callback that was bookmarked in the tracker
struct GhostTrackerEntry<Context, CbData, E> {
    expires: std::time::SystemTime,
    context: Context,
    cb: GhostCallback<dyn Any, Context, CbData, E>,
}

/// GhostTracker registers callbacks associated with request_ids
/// that can be triggered later when a response comes back indicating that id
pub struct GhostTracker<Context, CbData, E> {
    request_id_prefix: String,
    pending: HashMap<RequestId, GhostTrackerEntry<Context, CbData, E>>,
}

impl<Context: 'static, CbData: 'static, E: 'static> GhostTracker<Context, CbData, E> {
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
    pub fn bookmark<A: Any>(
        &mut self,
        timeout: std::time::Duration,
        context: Context,
        cb: GhostCallback<A, Context, CbData, E>,
    ) -> RequestId {
        let request_id = RequestId::with_prefix(&self.request_id_prefix);

        let cb: GhostCallback<dyn Any, _, _, _> = Box::new(move |a, ctx, data| {
            let a = a
                .downcast_mut::<A>()
                .expect("downcast Any to specific actor A");
            (cb)(a, ctx, data)
        });
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
    /// "owner" is meant to be the GhostActor (or other dynamic trait object) that is
    /// tracking for the call back, to get itself back in the callback and to an upcast
    pub fn handle(
        &mut self,
        request_id: RequestId,
        owner: &mut dyn Any,
        data: Result<CbData, E>,
    ) -> GhostResult<()> {
        match self.pending.remove(&request_id) {
            None => Err(GhostError::new(ErrorKind::RequestIdNotFound)),
            Some(entry) => (entry.cb)(owner, entry.context, GhostCallbackData::Response(data)),
        }
    }
}

#[cfg(test)]
mod tests {
    use detach::prelude::*;

    use super::*;
    #[derive(Debug)]
    struct TestCallbackData(String);
    #[derive(Debug)]
    struct TestContext(String);
    type TestError = String;
    struct TestTrackingActor {
        state: String,
        tracker: Detach<GhostTracker<TestContext, TestCallbackData, TestError>>,
    }

    use std::any::Any;

    impl TestTrackingActor {
        fn new(request_id_prefix: &str) -> Self {
            Self {
                state: "".into(),
                tracker: Detach::new(GhostTracker::new(request_id_prefix)),
            }
        }
        fn as_any(&mut self) -> &mut dyn Any {
            &mut *self
        }
    }

    #[test]
    fn test_ghost_tracker_should_bookmark_and_handle() {
        let mut actor = TestTrackingActor::new("test_request_id_prefix");
        let context = TestContext("some_context_data".into());

        let cb: GhostCallback<TestTrackingActor, TestContext, TestCallbackData, TestError> =
            Box::new(|dyn_me, context, callback_data| {
                // and we'll check that we got our context back too because we
                // might have used it to determine what to do here.
                assert_eq!(context.0, "some_context_data");
                if let GhostCallbackData::Response(Ok(TestCallbackData(payload))) = callback_data {
                    me.state = payload;
                }
                Ok(())
            });

        // lets bookmark a callback that should set our actors state to the value
        // of the callback response
        let req_id = actor.tracker.bookmark(
            // arbitrary timeout, we never call process in this test
            std::time::Duration::from_millis(2000),
            context,
            cb,
        );

        let entry = actor.tracker.pending.get(&req_id).unwrap();
        assert_eq!(entry.context.0, "some_context_data");

        // the state should be empty from the new
        assert_eq!(actor.state, "");
        // after handling
        detach_run!(&mut actor.tracker, |tracker| {
            let result = tracker.handle(
                req_id.clone(),
                actor.as_any(),
                Ok(TestCallbackData("here's the data!".into())),
            );
            assert_eq!("Ok(())", format!("{:?}", result))
        });
        assert_eq!(actor.state, "here's the data!");

        // try again and this time we should get that the request ID wasn't found
        detach_run!(&mut actor.tracker, |tracker| {
            let result = tracker.handle(
                req_id,
                actor.as_any(),
                Ok(TestCallbackData("here's the data again!".into())),
            );
            assert_eq!(
                "Err(GhostError(RequestIdNotFound))",
                format!("{:?}", result)
            )
        });
    }

    #[test]
    fn test_ghost_tracker_should_timeout() {
        let mut actor = TestTrackingActor::new("test_request_id_prefix");
        let context = TestContext("foo".into());
        let cb: GhostCallback<TestTrackingActor, TestContext, TestCallbackData, TestError> =
            Box::new(|me, _context, callback_data| {
                // when the timeout happens the callback should get
                // the timeout enum in the callback_data
                match callback_data {
                    GhostCallbackData::Timeout => me.state = "timed_out".into(),
                    _ => assert!(false),
                }
                Ok(())
            });
        let _req_id = actor
            .tracker
            .bookmark(std::time::Duration::from_millis(1), context, cb);
        assert_eq!(actor.tracker.pending.len(), 1);

        // wait 1 ms for the callback to have expired
        std::thread::sleep(std::time::Duration::from_millis(1));
        detach_run!(&mut actor.tracker, |tracker| {
            let result = tracker.process(actor.as_any());
            assert_eq!("Ok(())", format!("{:?}", result));
        });
        assert_eq!(actor.state, "timed_out");
        assert_eq!(actor.tracker.pending.len(), 0);
    }
}
