use holochain_tracing::Span;
use std::{collections::HashMap, sync::Arc};

use crate::*;

// TODO - should be 2000 or less but tests currently fail if below that
const DEFAULT_TIMEOUT_MS: u64 = 20000;

pub type GhostResponseCb<'lt, X, T> =
    Box<dyn FnOnce(Span, &mut X, GhostResult<T>) -> GhostResult<()> + 'lt + Send + Sync>;

/// this internal struct helps us keep track of the context and timeout
/// for a callback that was bookmarked in the tracker
struct GhostTrackerEntry<'lt, X, T> {
    span: Span,
    expires: std::time::Instant,
    cb: GhostResponseCb<'lt, X, T>,
}

enum GhostTrackerToInner<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync> {
    Bookmark(RequestId, GhostTrackerEntry<'lt, X, T>),
    Handle(Span, RequestId, T),
}

struct GhostTrackerInner<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync> {
    pending: HashMap<RequestId, GhostTrackerEntry<'lt, X, T>>,
    recv_inner: crossbeam_channel::Receiver<GhostTrackerToInner<'lt, X, T>>,
}

impl<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync> GhostTrackerInner<'lt, X, T> {
    fn new(recv_inner: crossbeam_channel::Receiver<GhostTrackerToInner<'lt, X, T>>) -> Self {
        Self {
            pending: HashMap::new(),
            recv_inner,
        }
    }

    /// trigger any periodic or delayed callbacks
    /// also check / cleanup any timeouts
    pub fn process(&mut self, user_data: &mut X) -> GhostResult<()> {
        self.priv_process_inner(user_data)?;
        self.priv_process_timeouts(user_data)?;
        Ok(())
    }

    fn priv_process_inner(&mut self, user_data: &mut X) -> GhostResult<()> {
        while let Ok(msg) = self.recv_inner.try_recv() {
            match msg {
                GhostTrackerToInner::Bookmark(request_id, entry) => {
                    self.priv_process_bookmark(request_id, entry)?;
                }
                GhostTrackerToInner::Handle(span, request_id, data) => {
                    self.priv_process_handle(span, user_data, request_id, data)?;
                }
            }
        }
        Ok(())
    }

    /// start tracking a pending bookmark request
    fn priv_process_bookmark(
        &mut self,
        request_id: RequestId,
        entry: GhostTrackerEntry<'lt, X, T>,
    ) -> GhostResult<()> {
        self.pending.insert(request_id, entry);
        Ok(())
    }

    /// match up a pending handle request with any pending bookmarks
    fn priv_process_handle(
        &mut self,
        span: Span,
        user_data: &mut X,
        request_id: RequestId,
        data: T,
    ) -> GhostResult<()> {
        match self.pending.remove(&request_id) {
            None => return Err(GhostErrorKind::RequestIdNotFound(String::new()).into()),
            Some(entry) => {
                (entry.cb)(span.child("process_handle"), user_data, Ok(data))?;
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
                    (entry.cb)(entry.span, user_data, Err("timeout".into()))?;
                }
            }
        }

        Ok(())
    }
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
    pub fn set_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// GhostTracker registers callbacks associated with request_ids
/// that can be triggered later when a response comes back indicating that id
pub struct GhostTracker<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync> {
    // just for ref count
    _inner: Arc<GhostMutex<GhostTrackerInner<'lt, X, T>>>,
    send_inner: crossbeam_channel::Sender<GhostTrackerToInner<'lt, X, T>>,
}

impl<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync> GhostTracker<'lt, X, T> {
    pub(crate) fn new<S: 'lt + GhostSystemRef<'lt>>(
        mut sys_ref: S,
        mut deep_user_data: DeepRef<'lt, X>,
    ) -> GhostResult<Self> {
        let (send_inner, recv_inner) = crossbeam_channel::unbounded();

        let inner = Arc::new(GhostMutex::new(GhostTrackerInner::new(recv_inner)));
        let weak_inner = Arc::downgrade(&inner);

        deep_user_data.push_cb(Box::new(move |weak_user_data| {
            let weak_inner_clone = weak_inner.clone();
            if let None = weak_inner.upgrade() {
                // we don't exist anymore, let this callback get dropped
                return Ok(false);
            }
            sys_ref.enqueue_processor(
                0,
                Box::new(move || match weak_inner_clone.upgrade() {
                    Some(strong_inner) => {
                        let mut strong_inner = strong_inner.lock();
                        match weak_user_data.upgrade() {
                            Some(strong_user_data) => {
                                let mut strong_user_data = strong_user_data.lock();

                                strong_inner
                                    .process(&mut *strong_user_data)
                                    .expect("tracker process error");

                                Ok(GhostProcessInstructions::default().set_should_continue(true))
                            }
                            None => Ok(GhostProcessInstructions::default()),
                        }
                    }
                    None => Ok(GhostProcessInstructions::default()),
                }),
            )?;
            Ok(true)
        }))?;

        Ok(Self {
            _inner: inner,
            send_inner,
        })
    }

    /// register a callback
    pub fn bookmark(
        &mut self,
        span: Span,
        cb: GhostResponseCb<'lt, X, T>,
    ) -> GhostResult<RequestId> {
        self.bookmark_with_options(span, cb, GhostTrackerBookmarkOptions::default())
    }

    /// register a callback, using a specific timeout instead of the default
    pub fn bookmark_with_options(
        &mut self,
        span: Span,
        cb: GhostResponseCb<'lt, X, T>,
        options: GhostTrackerBookmarkOptions,
    ) -> GhostResult<RequestId> {
        let request_id = RequestId::new();

        let timeout_ms = match options.timeout_ms {
            None => DEFAULT_TIMEOUT_MS,
            Some(timeout_ms) => timeout_ms,
        };

        self.send_inner.send(GhostTrackerToInner::Bookmark(
            request_id.clone(),
            GhostTrackerEntry {
                span,
                expires: std::time::Instant::now()
                    .checked_add(std::time::Duration::from_millis(timeout_ms))
                    .expect("can add timeout to SystemTime::now()"),
                cb,
            },
        ))?;

        Ok(request_id)
    }

    /// handle a response
    pub fn handle(&mut self, span: Span, request_id: RequestId, data: T) -> GhostResult<()> {
        self.send_inner.send(GhostTrackerToInner::Handle(
            span.child("tracker_handle"),
            request_id,
            data,
        ))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use holochain_tracing::Span;
    use std::sync::Arc;

    #[test]
    fn it_should_timeout() {
        #[derive(Debug)]
        struct Test {
            got_timeout: bool,
        }

        let test = Arc::new(GhostMutex::new(Test { got_timeout: false }));
        let mut deep = DeepRef::new();
        deep.set(Arc::downgrade(&test)).unwrap();

        let mut sys = SingleThreadedGhostSystem::new();
        let (_sys_ref, finalize) = sys.create_external_system_ref();
        finalize(Arc::downgrade(&test)).unwrap();

        let mut track: GhostTracker<Test, ()> = GhostTracker::new(sys.create_ref(), deep).unwrap();

        track
            .bookmark_with_options(
                holochain_tracing::test_span("test"),
                Box::new(|_span, me, response| {
                    assert_eq!(
                        "Err(GhostError(Other(\"timeout\")))",
                        &format!("{:?}", response)
                    );
                    me.got_timeout = true;
                    Ok(())
                }),
                GhostTrackerBookmarkOptions::default().set_timeout_ms(0),
            )
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1));

        sys.process().unwrap();

        assert!(test.lock().got_timeout);
    }

    #[test]
    fn it_should_respond() {
        #[derive(Debug)]
        struct Test {
            got_response: String,
        }

        let test = Arc::new(GhostMutex::new(Test {
            got_response: "".to_string(),
        }));
        let mut deep = DeepRef::new();
        deep.set(Arc::downgrade(&test)).unwrap();

        let mut sys = SingleThreadedGhostSystem::new();
        let (_sys_ref, finalize) = sys.create_external_system_ref();
        finalize(Arc::downgrade(&test)).unwrap();

        let mut track: GhostTracker<Test, String> =
            GhostTracker::new(sys.create_ref(), deep).unwrap();

        let rid = track
            .bookmark(
                holochain_tracing::test_span("test"),
                Box::new(|_span, me, response| {
                    me.got_response = format!("{:?}", response);
                    Ok(())
                }),
            )
            .unwrap();

        track
            .handle(
                holochain_tracing::test_span("test"),
                rid,
                "test-response".to_string(),
            )
            .unwrap();

        sys.process().unwrap();

        assert_eq!("Ok(\"test-response\")", &test.lock().got_response);
    }
}
