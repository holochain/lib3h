use holochain_tracing::Span;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};

use crate::{
    ghost_try_lock, GhostErrorKind, GhostProcessInstructions, GhostResult, GhostSystemRef,
    RequestId,
};

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
    sys_ref: GhostSystemRef<'lt>,
    weak_user_data: Weak<Mutex<X>>,
    // just for ref count
    _inner: Arc<Mutex<GhostTrackerInner<'lt, X, T>>>,
    send_bookmark: crossbeam_channel::Sender<(RequestId, GhostTrackerEntry<'lt, X, T>)>,
    send_handle: crossbeam_channel::Sender<(RequestId, T)>,
}

impl<'lt, X: 'lt + Send + Sync, T: 'lt + Send + Sync> GhostTracker<'lt, X, T> {
    pub fn new(mut sys_ref: GhostSystemRef<'lt>, weak_user_data: Weak<Mutex<X>>) -> Self {
        let (send_bookmark, recv_bookmark) = crossbeam_channel::unbounded();
        let (send_handle, recv_handle) = crossbeam_channel::unbounded();

        let inner = Arc::new(Mutex::new(GhostTrackerInner::new(
            recv_bookmark,
            recv_handle,
        )));
        let weak_inner = Arc::downgrade(&inner);

        let weak_user_data_clone = weak_user_data.clone();
        sys_ref
            .enqueue_processor(
                0,
                Box::new(move || match weak_inner.upgrade() {
                    Some(mut strong_inner) => match weak_user_data_clone.upgrade() {
                        Some(mut strong_user_data) => {
                            let mut strong_inner = ghost_try_lock(&mut strong_inner);
                            let mut strong_user_data = ghost_try_lock(&mut strong_user_data);

                            strong_inner
                                .process(&mut *strong_user_data)
                                .expect("tracker process error");

                            Ok(GhostProcessInstructions::default().set_should_continue(true))
                        }
                        None => Ok(GhostProcessInstructions::default()),
                    },
                    None => Ok(GhostProcessInstructions::default()),
                }),
            )
            .expect("can enqueue processor");

        Self {
            sys_ref,
            weak_user_data,
            _inner: inner,
            send_bookmark,
            send_handle,
        }
    }

    /// register a periodic task
    pub fn periodic_task(
        &mut self,
        start_delay_ms: u64,
        mut cb: GhostPeriodicCb<'lt, X>,
    ) -> GhostResult<()> {
        let weak_user_data_clone = self.weak_user_data.clone();
        self.sys_ref.enqueue_processor(
            start_delay_ms,
            Box::new(move || match weak_user_data_clone.upgrade() {
                Some(mut strong_user_data) => {
                    let mut strong_user_data = ghost_try_lock(&mut strong_user_data);
                    cb(&mut *strong_user_data)
                }
                None => Ok(GhostProcessInstructions::default()),
            }),
        )
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
    pub fn handle(&mut self, request_id: RequestId, data: T) -> GhostResult<()> {
        self.send_handle.send((request_id, data))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use holochain_tracing::Span;

    #[test]
    fn it_can_schedule_periodic() {
        #[derive(Debug)]
        struct Test {
            ticks: i32,
        }

        let test = Arc::new(Mutex::new(Test { ticks: 0 }));

        let mut sys = GhostSystem::new();

        let mut track: GhostTracker<Test, ()> =
            GhostTracker::new(sys.create_ref(), Arc::downgrade(&test));

        track
            .periodic_task(
                2,
                Box::new(|me| {
                    me.ticks += 1;
                    Ok(GhostProcessInstructions::default()
                        .set_should_continue(true)
                        .set_next_run_delay_ms(2))
                }),
            )
            .unwrap();

        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_millis(1));
            sys.process().unwrap();
        }

        let test = test.lock().unwrap();
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

        let test = Arc::new(Mutex::new(Test { got_timeout: false }));

        let mut sys = GhostSystem::new();

        let mut track: GhostTracker<Test, ()> =
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

        assert!(test.lock().unwrap().got_timeout);
    }

    #[test]
    fn it_should_respond() {
        #[derive(Debug)]
        struct Test {
            got_response: String,
        }

        let test = Arc::new(Mutex::new(Test {
            got_response: "".to_string(),
        }));

        let mut sys = GhostSystem::new();

        let mut track: GhostTracker<Test, String> =
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

        assert_eq!("Ok(\"test-response\")", &test.lock().unwrap().got_response);
    }
}
