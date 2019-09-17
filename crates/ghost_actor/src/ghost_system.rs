use std::sync::{Arc, RwLock};

use crate::*;

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
    _system_inner: Arc<RwLock<GhostSystemInner<'lt>>>,
}

impl<'lt> GhostSystemRef<'lt> {
    /// enqueue a new processor function for periodic execution
    pub fn enqueue_processor(&mut self, cb: GhostProcessCb<'lt>) -> GhostResult<()> {
        self.process_send.send(cb)?;
        Ok(())
    }
}

/// the main ghost system struct. Allows queueing new processor functions
/// and provides a process() function to actually execute them
pub struct GhostSystem<'lt> {
    process_send: crossbeam_channel::Sender<GhostProcessCb<'lt>>,
    system_inner: Arc<RwLock<GhostSystemInner<'lt>>>,
}

impl<'lt> GhostSystem<'lt> {
    /// create a new ghost system
    pub fn new() -> Self {
        let (process_send, process_recv) = crossbeam_channel::unbounded();
        Self {
            process_send,
            system_inner: Arc::new(RwLock::new(GhostSystemInner::new(process_recv))),
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

    /// enqueue a new processor function for periodic execution
    pub fn enqueue_processor(&mut self, cb: GhostProcessCb<'lt>) -> GhostResult<()> {
        self.process_send.send(cb)?;
        Ok(())
    }

    /// execute all queued processor functions
    pub fn process(&mut self) -> GhostResult<()> {
        self.system_inner
            .write()
            .expect("failed to obtain write lock")
            .process()
    }
}

pub trait TmpHandler<T>: TmpHandlerBase<T> {
    fn handle_a(&mut self, d: T);
    fn handle_b(&mut self, d: T);
}

pub trait TmpHandlerBase<T> {
    fn trigger(&mut self, d: T);
}

pub struct TmpHandlerConcrete;

impl TmpHandlerBase<String> for TmpHandlerConcrete {
    fn trigger(&mut self, d: String) {
        if d.as_bytes()[0] == b'a' {
            self.handle_a(d);
        } else {
            self.handle_b(d);
        }
    }
}

impl TmpHandler<String> for TmpHandlerConcrete {
    fn handle_a(&mut self, d: String) {
        println!("got a: {}", d);
    }

    fn handle_b(&mut self, d: String) {
        println!("got b: {}", d);
    }
}

pub struct GhostDock<'lt, U: 'lt> {
    _system: GhostSystemRef<'lt>,
    phantom_lifetime: std::marker::PhantomData<&'lt U>,
}

impl<'lt, U: 'lt> GhostDock<'lt, U> {
    pub fn new(system: GhostSystemRef<'lt>) -> Self {
        Self {
            _system: system,
            phantom_lifetime: std::marker::PhantomData,
        }
    }

    /// take an endpoint and a handler, and inflate a targetRef for output
    /// how do you handle owned vs ref dock_ref()?
    pub fn dock<T, H: TmpHandler<T>>(_handler: H) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_process() {
        let count = Arc::new(RwLock::new(0));
        let mut system = GhostSystem::new();

        {
            let count = count.clone();
            system
                .enqueue_processor(Box::new(move || {
                    let mut count = count.write().unwrap();
                    *count += 1;
                    if *count >= 2 {
                        false
                    } else {
                        true
                    }
                }))
                .unwrap();
        }

        // should increment
        system.process().unwrap();
        assert_eq!(1, *count.read().unwrap());
        // should increment
        system.process().unwrap();
        assert_eq!(2, *count.read().unwrap());
        // removed - should not increment
        system.process().unwrap();
        assert_eq!(2, *count.read().unwrap());
    }

    #[test]
    fn it_can_dock() {
        struct Z;

        let mut system = GhostSystem::new();
        let mut _dock: GhostDock<Z> = GhostDock::new(system.create_ref());

        system.process().unwrap();
    }
}
