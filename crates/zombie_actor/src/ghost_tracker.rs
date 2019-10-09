use holochain_tracing::Span;
use std::collections::HashMap;

use crate::{ghost_error::ErrorKind, Backtwrap, GhostError, GhostResult, RequestId, WorkWasDone};

const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(300000); // TODO - should be 2000 or less but tests currently fail if below that

/// a ghost request callback can be invoked with a response that was injected
/// into the system through the `handle` pathway, or to indicate a failure
/// such as a timeout
#[derive(Debug, Clone)]
pub enum GhostCallbackData<CbData: 'static, E: 'static> {
    Response(Result<CbData, E>),
    Timeout(Backtwrap),
}

/// definition for a ghost request callback
/// note, the callback can be registered as `'static` because the code
/// definition itself doesn't depend on any specific instance lifetime
/// if you want to mutate the state of a struct instance, pass it in
/// with the `handle` or `process` call.
/// (see detach crate for help with self refs)
pub type GhostCallback<UserData, CbData, E> =
    Box<dyn FnOnce(&mut UserData, GhostCallbackData<CbData, E>) -> GhostResult<()> + 'static>;

/// this internal struct helps us keep track of the context and timeout
/// for a callback that was bookmarked in the tracker
struct GhostTrackerEntry<UserData, CbData: 'static, E: 'static> {
    backtrace: Backtwrap,
    expires: std::time::SystemTime,
    cb: GhostCallback<UserData, CbData, E>,
}

#[derive(Debug, Clone)]
pub struct GhostTrackerBuilder {
    request_id_prefix: String,
    default_timeout: std::time::Duration,
}

impl Default for GhostTrackerBuilder {
    fn default() -> Self {
        Self {
            request_id_prefix: "".to_string(),
            default_timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl GhostTrackerBuilder {
    pub fn build<UserData, CbData: 'static, E: 'static>(self) -> GhostTracker<UserData, CbData, E> {
        GhostTracker {
            request_id_prefix: self.request_id_prefix,
            default_timeout: self.default_timeout,
            pending: HashMap::new(),
        }
    }

    pub fn request_id_prefix(mut self, request_id_prefix: &str) -> Self {
        self.request_id_prefix = request_id_prefix.to_string();
        self
    }

    pub fn default_timeout(mut self, default_timeout: std::time::Duration) -> Self {
        self.default_timeout = default_timeout;
        self
    }
}

/// GhostTracker registers callbacks associated with request_ids
/// that can be triggered later when a response comes back indicating that id
pub struct GhostTracker<UserData, CbData: 'static, E: 'static> {
    request_id_prefix: String,
    default_timeout: std::time::Duration,
    pending: HashMap<RequestId, GhostTrackerEntry<UserData, CbData, E>>,
}

#[derive(Debug, Clone)]
pub struct GhostTrackerBookmarkOptions {
    pub timeout: Option<std::time::Duration>,
}

impl Default for GhostTrackerBookmarkOptions {
    fn default() -> Self {
        Self { timeout: None }
    }
}

impl GhostTrackerBookmarkOptions {
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

impl<UserData, CbData: 'static, E: 'static> GhostTracker<UserData, CbData, E> {
    /// trigger any periodic or delayed callbacks
    /// also check / cleanup any timeouts
    pub fn process(&mut self, ga: &mut UserData) -> GhostResult<WorkWasDone> {
        let mut expired = Vec::new();

        let now = std::time::SystemTime::now();

        let did_work = !self.pending.is_empty();
        for (request_id, entry) in self.pending.iter() {
            if now > entry.expires {
                expired.push(request_id.clone())
            }
        }

        for request_id in expired {
            match self.pending.remove(&request_id) {
                None => (),
                Some(entry) => {
                    (entry.cb)(ga, GhostCallbackData::Timeout(entry.backtrace))?;
                }
            }
        }

        Ok(did_work.into())
    }

    /// register a callback
    pub fn bookmark(&mut self, span: Span, cb: GhostCallback<UserData, CbData, E>) -> RequestId {
        self.bookmark_options(span, cb, GhostTrackerBookmarkOptions::default())
    }

    /// register a callback, using a specific timeout instead of the default
    pub fn bookmark_options(
        &mut self,
        _span: Span,
        cb: GhostCallback<UserData, CbData, E>,
        options: GhostTrackerBookmarkOptions,
    ) -> RequestId {
        let request_id = RequestId::with_prefix(&self.request_id_prefix);

        let timeout = match options.timeout {
            None => self.default_timeout,
            Some(timeout) => timeout,
        };

        self.pending.insert(
            request_id.clone(),
            GhostTrackerEntry {
                backtrace: Backtwrap::new(),
                expires: std::time::SystemTime::now()
                    .checked_add(timeout)
                    .expect("can add timeout to SystemTime::now()"),
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
        owner: &mut UserData,
        data: Result<CbData, E>,
    ) -> GhostResult<()> {
        match self.pending.remove(&request_id) {
            None => {
                let msg = format!(
                    "{:?} in {:?}",
                    &request_id,
                    self.pending.keys().collect::<Vec<_>>()
                );
                Err(GhostError::new(ErrorKind::RequestIdNotFound(msg)))
            }
            Some(entry) => (entry.cb)(owner, GhostCallbackData::Response(data)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use detach::prelude::*;
    use holochain_tracing::test_span;

    type TestError = String;

    #[derive(Debug)]
    pub struct TestCallbackData(pub String);

    struct TestTrackingActor {
        state: String,
        tracker: Detach<GhostTracker<TestTrackingActor, TestCallbackData, TestError>>,
    }

    impl TestTrackingActor {
        fn new(request_id_prefix: &str) -> Self {
            let tracker_builder =
                GhostTrackerBuilder::default().request_id_prefix(request_id_prefix);
            Self {
                state: "".into(),
                tracker: Detach::new(tracker_builder.build()),
            }
        }
    }

    #[test]
    fn test_ghost_tracker_should_bookmark_and_handle() {
        let mut actor = TestTrackingActor::new("test_request_id_prefix");
        let span = test_span("some_context_data");

        let cb: GhostCallback<TestTrackingActor, TestCallbackData, TestError> =
            Box::new(|me, callback_data| {
                if let GhostCallbackData::Response(Ok(TestCallbackData(payload))) = callback_data {
                    me.state = payload;
                }
                Ok(())
            });

        // lets bookmark a callback that should set our actors state to the value
        // of the callback response
        let req_id = actor.tracker.bookmark(span, cb);

        let _entry = actor.tracker.pending.get(&req_id).unwrap();

        // the state should be empty from the new
        assert_eq!(actor.state, "");
        // after handling
        detach_run!(&mut actor.tracker, |tracker| {
            let result = tracker.handle(
                req_id.clone(),
                &mut actor,
                Ok(TestCallbackData("here's the data!".into())),
            );
            assert_eq!("Ok(())", format!("{:?}", result))
        });
        assert_eq!(actor.state, "here's the data!");

        // try again and this time we should get that the request ID wasn't found
        detach_run!(&mut actor.tracker, |tracker| {
            let result = tracker.handle(
                req_id,
                &mut actor,
                Ok(TestCallbackData("here's the data again!".into())),
            );
            assert_eq!(
                b"Err(GhostError(RequestIdNotFound",
                &format!("{:?}", result).as_bytes()[..32],
            )
        });
    }

    #[test]
    fn test_ghost_tracker_should_timeout() {
        let mut actor = TestTrackingActor::new("test_request_id_prefix");
        let span = test_span("foo");
        let cb: GhostCallback<TestTrackingActor, TestCallbackData, TestError> =
            Box::new(|me, callback_data| {
                // when the timeout happens the callback should get
                // the timeout enum in the callback_data
                match callback_data {
                    GhostCallbackData::Timeout(_) => me.state = "timed_out".into(),
                    _ => assert!(false),
                }
                Ok(())
            });
        let _req_id = actor.tracker.bookmark_options(
            span,
            cb,
            GhostTrackerBookmarkOptions::default().timeout(std::time::Duration::from_millis(1)),
        );
        assert_eq!(actor.tracker.pending.len(), 1);

        // wait 1 ms for the callback to have expired
        std::thread::sleep(std::time::Duration::from_millis(1));
        detach_run!(&mut actor.tracker, |tracker| {
            let result = tracker.process(&mut actor);
            assert_eq!("Ok(WorkWasDone(true))", format!("{:?}", result));
        });
        assert_eq!(actor.state, "timed_out");
        assert_eq!(actor.tracker.pending.len(), 0);
    }
}
