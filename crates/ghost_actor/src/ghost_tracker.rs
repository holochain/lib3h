use holochain_tracing::Span;
use std::sync::{Arc, Weak, Mutex};
use std::collections::HashMap;

use crate::{ghost_try_lock, GhostErrorKind, GhostResult, RequestId, GhostSystemRef, GhostProcessInstructions};

// TODO - should be 2000 or less but tests currently fail if below that
const DEFAULT_TIMEOUT_MS: u64 = 20000;

pub type GhostResponseCb<'lt, X, T> =
    Box<dyn FnOnce(&mut X, GhostResult<T>) -> GhostResult<()> + 'lt + Send + Sync>;

pub type GhostPeriodicCb<'lt, X> =
    Box<dyn FnMut(&mut X) -> GhostProcessInstructions + 'lt + Send + Sync>;

/// this internal struct helps us keep track of the context and timeout
/// for a callback that was bookmarked in the tracker
struct GhostTrackerEntry<'lt, X, T> {
    expires: std::time::Instant,
    cb: GhostResponseCb<'lt, X, T>,
}

#[derive(Debug, Clone)]
pub struct GhostTrackerBuilder {
    request_id_prefix: String,
    default_timeout_ms: u64,
}

impl Default for GhostTrackerBuilder {
    fn default() -> Self {
        Self {
            request_id_prefix: "".to_string(),
            default_timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }
}

impl GhostTrackerBuilder {
    pub fn build<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync>(self, sys_ref: GhostSystemRef<'lt>, weak_user_data: Weak<Mutex<X>>) -> GhostTracker<'lt, X, T> {
        GhostTracker::new(sys_ref, weak_user_data, self.request_id_prefix, self.default_timeout_ms)
    }

    pub fn request_id_prefix(mut self, request_id_prefix: &str) -> Self {
        self.request_id_prefix = request_id_prefix.to_string();
        self
    }

    pub fn default_timeout_ms(mut self, default_timeout_ms: u64) -> Self {
        self.default_timeout_ms = default_timeout_ms;
        self
    }
}

struct GhostTrackerInner<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync> {
    pending: HashMap<RequestId, GhostTrackerEntry<'lt, X, T>>,
    recv_bookmark: crossbeam_channel::Receiver<(RequestId, GhostTrackerEntry<'lt, X, T>)>,
    recv_handle: crossbeam_channel::Receiver<(RequestId, T)>,
}

impl<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync> GhostTrackerInner<'lt, X, T> {
    fn new(
        recv_bookmark: crossbeam_channel::Receiver<(RequestId, GhostTrackerEntry<'lt, X, T>)>,
        recv_handle: crossbeam_channel::Receiver<(RequestId, T)>,
    ) -> Self {
        Self {
            pending: HashMap::new(),
            recv_bookmark,
            recv_handle,
        }
    }

    /// trigger any periodic or delayed callbacks
    /// also check / cleanup any timeouts
    pub fn process(&mut self, user_data: &mut X) -> GhostResult<()> {
        // order is important so we can test with fewest process() calls
        self.priv_process_bookmarks()?;
        self.priv_process_handle(user_data)?;
        self.priv_process_timeouts(user_data)?;
        Ok(())
    }

    /// start tracking any pending bookmark requests
    fn priv_process_bookmarks(&mut self) -> GhostResult<()> {
        while let Ok((request_id, entry)) = self.recv_bookmark.try_recv() {
            self.pending.insert(request_id, entry);
        }
        Ok(())
    }

    /// match up any pending handle requests with pending bookmarks
    fn priv_process_handle(&mut self, user_data: &mut X) -> GhostResult<()> {
        while let Ok((request_id, data)) = self.recv_handle.try_recv() {
            match self.pending.remove(&request_id) {
                None => return Err(GhostErrorKind::RequestIdNotFound("".to_string()).into()),
                Some(entry) => {
                    (entry.cb)(user_data, Ok(data))?;
                }
            }
        }
        Ok(())
    }

    /// if there are any expired bookmarks, clean them up
    fn priv_process_timeouts(&mut self, user_data: &mut X) -> GhostResult<()> {
        let mut expired = Vec::new();

        let now = std::time::Instant::now();

        for (request_id, entry) in self.pending.iter() {
            if now > entry.expires {
                expired.push(request_id.clone())
            }
        }

        for request_id in expired {
            match self.pending.remove(&request_id) {
                None => (),
                Some(entry) => {
                    (entry.cb)(user_data, Err("timeout".into()))?;
                }
            }
        }

        Ok(())
    }
}

/// GhostTracker registers callbacks associated with request_ids
/// that can be triggered later when a response comes back indicating that id
pub struct GhostTracker<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync> {
    // just a ref count
    _sys_ref: GhostSystemRef<'lt>,
    request_id_prefix: String,
    default_timeout_ms: u64,
    // just for ref count
    _inner: Arc<Mutex<GhostTrackerInner<'lt, X, T>>>,
    send_bookmark: crossbeam_channel::Sender<(RequestId, GhostTrackerEntry<'lt, X, T>)>,
    send_handle: crossbeam_channel::Sender<(RequestId, T)>,
}

#[derive(Debug, Clone)]
pub struct GhostTrackerBookmarkOptions {
    pub timeout_ms: Option<u64>,
}

impl Default for GhostTrackerBookmarkOptions {
    fn default() -> Self {
        Self { timeout_ms: None }
    }
}

impl GhostTrackerBookmarkOptions {
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

impl<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync> GhostTracker<'lt, X, T> {
    fn new(
        mut sys_ref: GhostSystemRef<'lt>,
        weak_user_data: Weak<Mutex<X>>,
        request_id_prefix: String,
        default_timeout_ms: u64,
    ) -> Self {
        let (send_bookmark, recv_bookmark) = crossbeam_channel::unbounded();
        let (send_handle, recv_handle) = crossbeam_channel::unbounded();

        let inner = Arc::new(Mutex::new(GhostTrackerInner::new(
            recv_bookmark,
            recv_handle,
        )));
        let weak_inner = Arc::downgrade(&inner);

        sys_ref.enqueue_processor(0, Box::new(move || match weak_inner.upgrade() {
            Some(mut strong_inner) => match weak_user_data.upgrade() {
                Some(mut strong_user_data) => {
                    let mut strong_inner = ghost_try_lock(&mut strong_inner);
                    let mut strong_user_data = ghost_try_lock(&mut strong_user_data);

                    strong_inner.process(&mut *strong_user_data)
                        .expect("tracker process error");

                    GhostProcessInstructions::default()
                        .set_should_continue(true)
                }
                None => GhostProcessInstructions::default(),
            }
            None => GhostProcessInstructions::default(),
        })).expect("can enqueue processor");

        Self {
            _sys_ref: sys_ref,
            request_id_prefix,
            default_timeout_ms,
            _inner: inner,
            send_bookmark,
            send_handle,
        }
    }

    /// register a callback
    pub fn bookmark(&mut self, span: Span, cb: GhostResponseCb<'lt, X, T>) -> GhostResult<RequestId> {
        self.bookmark_options(span, cb, GhostTrackerBookmarkOptions::default())
    }

    /// register a callback, using a specific timeout instead of the default
    pub fn bookmark_options(
        &mut self,
        _span: Span,
        cb: GhostResponseCb<'lt, X, T>,
        options: GhostTrackerBookmarkOptions,
    ) -> GhostResult<RequestId> {
        let request_id = RequestId::with_prefix(&self.request_id_prefix);

        let timeout_ms = match options.timeout_ms {
            None => self.default_timeout_ms,
            Some(timeout_ms) => timeout_ms,
        };

        self.send_bookmark.send((
            request_id.clone(),
            GhostTrackerEntry {
                expires: std::time::Instant::now()
                    .checked_add(std::time::Duration::from_millis(timeout_ms))
                    .expect("can add timeout to SystemTime::now()"),
                cb,
            },
        ))?;

        Ok(request_id)
    }

    /// handle a response
    pub fn handle(
        &mut self,
        request_id: RequestId,
        data: T,
    ) -> GhostResult<()> {
        self.send_handle.send((
            request_id,
            data,
        ))?;

        Ok(())
    }
}

/*
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
*/
