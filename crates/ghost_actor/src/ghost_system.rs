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

    /// execute all queued processor functions
    pub fn process(&mut self) -> GhostResult<()> {
        self.system_inner
            .write()
            .expect("failed to obtain write lock")
            .process()
    }

    //pub fn spawn<X: 'lt,
}

pub type GhostHandlerCb<'lt, T> = Box<dyn FnOnce(T) -> GhostResult<()> + 'lt + Send + Sync>;

pub type GhostResponseCb<'lt, X, T> =
    Box<dyn FnOnce(&mut X, GhostResult<T>) -> GhostResult<()> + 'lt + Send + Sync>;

pub struct TestActorHandler<'lt, X: 'lt> {
    phantom: std::marker::PhantomData<&'lt X>,
    pub handle_event_to_actor_print: Box<dyn FnMut(X, String) -> GhostResult<()> + 'lt>,
    pub handle_request_to_actor_add_1: Box<dyn FnMut(X, i32, GhostHandlerCb<'lt, Result<i32, ()>>) -> GhostResult<()> + 'lt>,
}

pub struct TestOwnerHandler<'lt, X: 'lt> {
    phantom: std::marker::PhantomData<&'lt X>,
    pub handle_event_to_owner_print: Box<dyn FnMut(X, String) -> GhostResult<()> + 'lt>,
    pub handle_request_to_owner_sub_1: Box<dyn FnMut(X, i32, GhostHandlerCb<'lt, Result<i32, ()>>) -> GhostResult<()> + 'lt>,
}

pub trait TestActorRef<'lt, X: 'lt> {
    fn event_to_actor_print(&mut self, message: String) -> GhostResult<()>;
    fn request_to_actor_add_1(&mut self, message: i32, cb: GhostResponseCb<'lt, X, Result<i32, ()>>) -> GhostResult<()>;
}

pub trait TestOwnerRef<'lt, X: 'lt> {
    fn event_to_owner_print(&mut self, message: String) -> GhostResult<()>;
    fn request_to_owner_sub_1(&mut self, message: i32, cb: GhostResponseCb<'lt, X, Result<i32, ()>>) -> GhostResult<()>;
}

pub trait GhostActor<'lt, OwnerRefType: 'lt, ActorHandlerType: 'lt> {
    fn actor_init(system: GhostSystemRef<'lt>, owner_ref: OwnerRefType) -> GhostResult<ActorHandlerType>;
}

struct TestActor;

impl<'lt, O: 'lt + TestOwnerRef<'lt, TestActor>> GhostActor<'lt, O, TestActorHandler<'lt, TestActor>> for TestActor {
    fn actor_init(_system: GhostSystemRef<'lt>, mut owner_ref: O) -> GhostResult<TestActorHandler<'lt, TestActor>> {
        owner_ref.event_to_owner_print("message from actor".to_string())?;
        owner_ref.request_to_owner_sub_1(42, Box::new(|_me, result| {
            println!("got sub from owner: 42 - 1 = {:?}", result);
            Ok(())
        }))?;

        Ok(TestActorHandler {
            phantom: std::marker::PhantomData,
            handle_event_to_actor_print: Box::new(|_me, message| {
                println!("actor print: {}", message);
                Ok(())
            }),
            handle_request_to_actor_add_1: Box::new(|_me, message, cb| {
                cb(Ok(message + 1))
            }),
        })
    }
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
            system.create_ref()
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
        struct Z {
            pub s: String,
        }

        let mut system = GhostSystem::new();
        //let mut _dock: GhostDock<Z> = GhostDock::new(system.create_ref());

        // --- demo dock --- ///

        /*
        struct MyActor;

        impl GhostActor for MyActor {
            fn actor_init(dock: GhostDock, owner_ref: ??) -> Handler {
                // TODO - store dock
                // TODO - store owner_ref

                // dock can spawn sub-actor

                owner_ref.event_to_owner_print("bla".to_string())?;
                owner_ref.request_to_owner_sub_1(42, |me, message| {
                    println!("got: {}", message);
                    Ok(())
                })?;

                Handler {
                    handle_event_to_actor_print: |me, message| {
                        println!("{}", message);
                        Ok(())
                    },
                    handle_request_to_owner_sub_1: |me, message, cb| {
                        cb(Ok(message + 1))
                    },
                }
            }
        }

        let actor_ref = dock.spawn(MyActor::new(), Handler {
            handle_event_to_owner_print: |me, message| {
                println!("{}", message);
                Ok(())
            },
            handle_request_to_owner_sub_1: |me, message, cb| {
                cb(Ok(message - 1))
            },
        });

        actor_ref.event_to_actor_print("bla".to_string())?;
        actor_ref.request_to_actor_add_1(42, |me, message| {
            println!("got: {}", message);
            Ok(())
        })?;
        */

        // --- demo dock --- ///

        system.process().unwrap();

        pub struct ZZHandler<F1, F2>
        where
            F1: FnMut(String),
            F2: FnMut(i32),
        {
            pub f1: F1,
            pub f2: F2,
        }

        impl Z {
            pub fn go(&mut self) {
                self.s = "funk".to_string();
                let mut zz = ZZHandler {
                    f1: |s| {
                        println!("str: {} {}", s, self.s);
                    },
                    f2: |i| {
                        println!("int: {} {}", i, self.s);
                    },
                };

                (zz.f1)("test".to_string());
                (zz.f2)(42);
            }
        }

        let mut z = Z { s: "".to_string() };
        z.go();
    }
}
