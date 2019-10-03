use crate::*;
use std::sync::{Arc, Mutex, Weak};

/// typedef for a periodic process callback
pub type GhostProcessCb<'lt> = Box<dyn FnMut() -> bool + 'lt + Send + Sync>;

/// inner struct trackes queued process callbacks
struct GhostSystemInner<'lt> {
    process_recv: crossbeam_channel::Receiver<GhostProcessCb<'lt>>,
    process_queue: Vec<GhostProcessCb<'lt>>,
}

impl<'lt> GhostSystemInner<'lt> {
    /// new inner system
    fn new(process_recv: crossbeam_channel::Receiver<GhostProcessCb<'lt>>) -> Self {
        Self {
            process_recv,
            process_queue: Vec::new(),
        }
    }

    /// first, check for new process functions,
    /// then, actually loop through the queued process functions
    /// keep them for next time if they return true
    fn process(&mut self) -> GhostResult<()> {
        while let Ok(item) = self.process_recv.try_recv() {
            self.process_queue.push(item);
        }
        for mut item in self.process_queue.drain(..).collect::<Vec<_>>() {
            if item() {
                self.process_queue.push(item);
            }
        }
        Ok(())
    }
}

/// Ref that allows queueing of new process functions
/// but does not have the ability to actually run process
#[derive(Clone)]
pub struct GhostSystemRef<'lt> {
    process_send: crossbeam_channel::Sender<GhostProcessCb<'lt>>,
    // just a refcount
    _system_inner: Arc<Mutex<GhostSystemInner<'lt>>>,
}

impl<'lt> GhostSystemRef<'lt> {
    /// enqueue a new processor function for periodic execution
    pub fn enqueue_processor(&mut self, cb: GhostProcessCb<'lt>) -> GhostResult<()> {
        self.process_send.send(cb)?;
        Ok(())
    }

    /// spawn / manage a new actor
    pub fn spawn<
        'a,
        X: 'lt + Send + Sync,
        P: GhostProtocol,
        A: 'lt + GhostActor<'lt, P, A>,
        H: 'lt + GhostHandler<'lt, X, P>,
    >(
        &'a mut self,
        user_data: Weak<Mutex<X>>,
        actor: A,
        handler: H,
    ) -> GhostResult<GhostEndpointRef<'lt, X, A, P, H>> {
        let (s1, r1) = crossbeam_channel::unbounded();
        let (s2, r2) = crossbeam_channel::unbounded();

        let mut actor = Arc::new(Mutex::new(actor));

        let inflator = GhostInflator {
            phantom_a: std::marker::PhantomData,
            phantom_b: std::marker::PhantomData,
            system_ref: self.clone(),
            sender: s2,
            receiver: r1,
            weak_ref: Arc::downgrade(&actor),
        };

        ghost_try_lock(&mut actor).actor_init(inflator)?;

        let weak_ref = Arc::downgrade(&actor);

        self.enqueue_processor(Box::new(move || match weak_ref.upgrade() {
            Some(mut strong_actor) => {
                let mut strong_actor = ghost_try_lock(&mut strong_actor);
                match strong_actor.process() {
                    Ok(()) => true,
                    Err(e) => panic!("actor.process() error: {:?}", e),
                }
            }
            None => false,
        }))?;

        GhostEndpointRef::new(s1, r2, self, actor, user_data, handler)
    }
}

/// the main ghost system struct. Allows queueing new processor functions
/// and provides a process() function to actually execute them
pub struct GhostSystem<'lt> {
    process_send: crossbeam_channel::Sender<GhostProcessCb<'lt>>,
    system_inner: Arc<Mutex<GhostSystemInner<'lt>>>,
}

impl<'lt> GhostSystem<'lt> {
    /// create a new ghost system
    pub fn new() -> Self {
        let (process_send, process_recv) = crossbeam_channel::unbounded();
        Self {
            process_send,
            system_inner: Arc::new(Mutex::new(GhostSystemInner::new(process_recv))),
        }
    }

    /// get a GhostSystemRef capable of enqueueing new processor functions
    /// without creating any deadlocks
    pub fn create_ref(&self) -> GhostSystemRef<'lt> {
        GhostSystemRef {
            process_send: self.process_send.clone(),
            _system_inner: self.system_inner.clone(),
        }
    }

    /// execute all queued processor functions
    pub fn process(&mut self) -> GhostResult<()> {
        ghost_try_lock(&mut self.system_inner).process()
    }
}
