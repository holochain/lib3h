use holochain_tracing::Span;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

use crate::*;

// TODO - should be 2000 or less but tests currently fail if below that
const DEFAULT_TIMEOUT_MS: u64 = 20000;

pub type GhostResponseCb<'lt, X, T> =
    Box<dyn FnOnce(&mut X, GhostResult<T>) -> GhostResult<()> + 'lt + Send + Sync>;

pub type GhostPeriodicCb<'lt, X> =
    Box<dyn FnMut(&mut X) -> GhostResult<GhostProcessInstructions> + 'lt + Send + Sync>;

/// this internal struct helps us keep track of the context and timeout
/// for a callback that was bookmarked in the tracker
struct GhostTrackerEntry<'lt, X, T> {
    expires: std::time::Instant,
    cb: GhostResponseCb<'lt, X, T>,
}

enum GhostTrackerToInner<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync> {
    Bookmark(RequestId, GhostTrackerEntry<'lt, X, T>),
    Handle(RequestId, T),
    Periodic(u64, GhostPeriodicCb<'lt, X>),
}

struct GhostTrackerInner<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync, S: GhostSystemRef<'lt>> {
    weak_user_data: Weak<GhostMutex<X>>,
    pending: HashMap<RequestId, GhostTrackerEntry<'lt, X, T>>,
    recv_inner: crossbeam_channel::Receiver<GhostTrackerToInner<'lt, X, T>>,
    sys_ref: S,
}

impl<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync, S: GhostSystemRef<'lt>>
    GhostTrackerInner<'lt, X, T, S>
{
    fn new(
        sys_ref: S,
        weak_user_data: Weak<GhostMutex<X>>,
        recv_inner: crossbeam_channel::Receiver<GhostTrackerToInner<'lt, X, T>>,
    ) -> Self {
        Self {
            sys_ref,
            weak_user_data,
            pending: HashMap::new(),
            recv_inner,
        }
    }

    /// trigger any periodic or delayed callbacks
    /// also check / cleanup any timeouts
    pub fn process(&mut self, user_data: &mut X) -> GhostResult<()> {
        if self.priv_process_inner(user_data)? {
            // we got new user_data...
            // we can't continue until the next process() call
            return Ok(());
        }
        self.priv_process_timeouts(user_data)?;
        Ok(())
    }

    fn priv_process_inner(&mut self, user_data: &mut X) -> GhostResult<bool> {
        while let Ok(msg) = self.recv_inner.try_recv() {
            match msg {
                GhostTrackerToInner::Bookmark(request_id, entry) => {
                    self.priv_process_bookmark(request_id, entry)?;
                }
                GhostTrackerToInner::Handle(request_id, data) => {
                    self.priv_process_handle(user_data, request_id, data)?;
                }
                GhostTrackerToInner::Periodic(start_delay_ms, cb) => {
                    self.priv_process_periodic(start_delay_ms, cb)?;
                }
            }
        }
        Ok(false)
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
        user_data: &mut X,
        request_id: RequestId,
        data: T,
    ) -> GhostResult<()> {
        match self.pending.remove(&request_id) {
            None => return Err(GhostErrorKind::RequestIdNotFound("".to_string()).into()),
            Some(entry) => {
                (entry.cb)(user_data, Ok(data))?;
            }
        }
        Ok(())
    }

    /// queue up a periodic processing task
    fn priv_process_periodic(
        &mut self,
        start_delay_ms: u64,
        mut cb: GhostPeriodicCb<'lt, X>,
    ) -> GhostResult<()> {
        let weak_user_data_clone = self.weak_user_data.clone();
        self.sys_ref.enqueue_processor(
            start_delay_ms,
            Box::new(move || match weak_user_data_clone.upgrade() {
                Some(strong_user_data) => {
                    let mut strong_user_data = strong_user_data.lock();
                    cb(&mut *strong_user_data)
                }
                None => Ok(GhostProcessInstructions::default()),
            }),
        )
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
pub struct GhostTracker<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync, S: GhostSystemRef<'lt>> {
    // just for ref count
    _inner: Arc<GhostMutex<GhostTrackerInner<'lt, X, T, S>>>,
    send_inner: crossbeam_channel::Sender<GhostTrackerToInner<'lt, X, T>>,
}

impl<
        'lt,
        X: 'lt + Send + Sync,
        T: 'lt + Send + Sync,
        S: 'lt + GhostSystemRef<'lt>,
    > GhostTracker<'lt, X, T, S>
{
    pub fn new(mut sys_ref: S, weak_user_data: Weak<GhostMutex<X>>) -> Self {
        let (send_inner, recv_inner) = crossbeam_channel::unbounded();

        let inner = Arc::new(GhostMutex::new(GhostTrackerInner::new(
            sys_ref.clone(),
            weak_user_data,
            recv_inner,
        )));
        let weak_inner = Arc::downgrade(&inner);

        sys_ref
            .enqueue_processor(
                0,
                Box::new(move || match weak_inner.upgrade() {
                    Some(strong_inner) => {
                        let mut strong_inner = strong_inner.lock();
                        match strong_inner.weak_user_data.upgrade() {
                            Some(strong_user_data) => {
                                let mut strong_user_data = strong_user_data.lock();

                                strong_inner
                                    .process(&mut *strong_user_data)
                                    .expect("tracker process error");

                                Ok(GhostProcessInstructions::default().set_should_continue(true))
                            }
                            None => {
                                // we don't have any user_data, next time?
                                Ok(GhostProcessInstructions::default().set_should_continue(true))
                            }
                        }
                    }
                    None => Ok(GhostProcessInstructions::default()),
                }),
            )
            .expect("can enqueue processor");

        Self {
            _inner: inner,
            send_inner,
        }
    }

    /// register a periodic task
    pub fn periodic_task(
        &mut self,
        start_delay_ms: u64,
        cb: GhostPeriodicCb<'lt, X>,
    ) -> GhostResult<()> {
        self.send_inner
            .send(GhostTrackerToInner::Periodic(start_delay_ms, cb))?;
        Ok(())
    }

    /// register a callback
    pub fn bookmark(
        &mut self,
        span: Span,
        cb: GhostResponseCb<'lt, X, T>,
    ) -> GhostResult<RequestId> {
        self.bookmark_options(span, cb, GhostTrackerBookmarkOptions::default())
    }

    /// register a callback, using a specific timeout instead of the default
    pub fn bookmark_options(
        &mut self,
        _span: Span,
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
                expires: std::time::Instant::now()
                    .checked_add(std::time::Duration::from_millis(timeout_ms))
                    .expect("can add timeout to SystemTime::now()"),
                cb,
            },
        ))?;

        Ok(request_id)
    }

    /// handle a response
    pub fn handle(&mut self, request_id: RequestId, data: T) -> GhostResult<()> {
        self.send_inner
            .send(GhostTrackerToInner::Handle(request_id, data))?;
        Ok(())
    }

    /// replace user data
    pub fn set_user_data(&mut self, user_data: Weak<GhostMutex<X>>) -> GhostResult<()> {
        self._inner.lock().weak_user_data = user_data;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use holochain_tracing::Span;
    use std::sync::Arc;

    #[test]
    fn it_can_schedule_periodic() {
        #[derive(Debug)]
        struct Test {
            ticks: i32,
        }

        let test = Arc::new(GhostMutex::new(Test { ticks: 0 }));

        let mut sys = SingleThreadedGhostSystem::new();

        let mut track: GhostTracker<Test, (), SingleThreadedGhostSystemRef> =
            GhostTracker::new(sys.create_ref(), Arc::downgrade(&test));

        track
            .periodic_task(
                20,
                Box::new(|me| {
                    me.ticks += 1;
                    Ok(GhostProcessInstructions::default()
                        .set_should_continue(true)
                        .set_next_run_delay_ms(40))
                }),
            )
            .unwrap();

        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            sys.process().unwrap();
        }

        let test = test.lock();
        println!("got {:?}", *test);
        assert!(test.ticks > 0);
        assert!(test.ticks < 9);
    }

    #[test]
    fn it_should_timeout() {
        #[derive(Debug)]
        struct Test {
            got_timeout: bool,
        }

        let test = Arc::new(GhostMutex::new(Test { got_timeout: false }));

        let mut sys = SingleThreadedGhostSystem::new();

        let mut track: GhostTracker<Test, (), SingleThreadedGhostSystemRef> =
            GhostTracker::new(sys.create_ref(), Arc::downgrade(&test));

        track
            .bookmark_options(
                Span::fixme(),
                Box::new(|me, response| {
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

        let mut sys = SingleThreadedGhostSystem::new();

        let mut track: GhostTracker<Test, String, SingleThreadedGhostSystemRef> =
            GhostTracker::new(sys.create_ref(), Arc::downgrade(&test));

        let rid = track
            .bookmark(
                Span::fixme(),
                Box::new(|me, response| {
                    me.got_response = format!("{:?}", response);
                    Ok(())
                }),
            )
            .unwrap();

        track.handle(rid, "test-response".to_string()).unwrap();

        sys.process().unwrap();

        assert_eq!("Ok(\"test-response\")", &test.lock().got_response);
    }
}
