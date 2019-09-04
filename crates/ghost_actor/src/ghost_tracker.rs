use std::collections::HashMap;

use crate::{ghost_error::ErrorKind, GhostError, GhostResult, RequestId};

const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(2000);

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
pub type GhostCallback<UserData, Context, CbData, E> =
    Box<dyn Fn(&mut UserData, Context, GhostCallbackData<CbData, E>) -> GhostResult<()> + 'static>;

/// this internal struct helps us keep track of the context and timeout
/// for a callback that was bookmarked in the tracker
struct GhostTrackerEntry<UserData, Context, CbData, E> {
    expires: std::time::SystemTime,
    context: Context,
    cb: GhostCallback<UserData, Context, CbData, E>,
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
    pub fn build<UserData, Context, CbData, E>(self) -> GhostTracker<UserData, Context, CbData, E> {
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
pub struct GhostTracker<UserData, Context, CbData, E> {
    request_id_prefix: String,
    default_timeout: std::time::Duration,
    pending: HashMap<RequestId, GhostTrackerEntry<UserData, Context, CbData, E>>,
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

impl<UserData, Context: 'static, CbData: 'static, E: 'static>
    GhostTracker<UserData, Context, CbData, E>
{
    /// trigger any periodic or delayed callbacks
    /// also check / cleanup any timeouts
    pub fn process(&mut self, ga: &mut UserData) -> GhostResult<()> {
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
        context: Context,
        cb: GhostCallback<UserData, Context, CbData, E>,
    ) -> RequestId {
        self.bookmark_options(context, cb, GhostTrackerBookmarkOptions::default())
    }

    /// register a callback, using a specific timeout instead of the default
    pub fn bookmark_options(
        &mut self,
        context: Context,
        cb: GhostCallback<UserData, Context, CbData, E>,
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
        owner: &mut UserData,
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
        tracker: Detach<GhostTracker<TestTrackingActor, TestContext, TestCallbackData, TestError>>,
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
        let context = TestContext("some_context_data".into());

        let cb: GhostCallback<TestTrackingActor, TestContext, TestCallbackData, TestError> =
            Box::new(|me, context, callback_data| {
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
        let req_id = actor.tracker.bookmark(context, cb);

        let entry = actor.tracker.pending.get(&req_id).unwrap();
        assert_eq!(entry.context.0, "some_context_data");

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
        let _req_id = actor.tracker.bookmark_options(
            context,
            cb,
            GhostTrackerBookmarkOptions::default().timeout(std::time::Duration::from_millis(1)),
        );
        assert_eq!(actor.tracker.pending.len(), 1);

        // wait 1 ms for the callback to have expired
        std::thread::sleep(std::time::Duration::from_millis(1));
        detach_run!(&mut actor.tracker, |tracker| {
            let result = tracker.process(&mut actor);
            assert_eq!("Ok(())", format!("{:?}", result));
        });
        assert_eq!(actor.state, "timed_out");
        assert_eq!(actor.tracker.pending.len(), 0);
    }
}
