use std::sync::{Arc, RwLock, Weak};

// --- demo --- ///

/*
struct MyActor;

impl GhostActor for MyActor {
    fn actor_init(inflator: GhostInflator) -> GhostResult<()> {
        let (system_ref, owner_ref) = inflator.inflate(Handler {
            handle_event_to_actor_print: |me, message| {
                println!("{}", message);
                Ok(())
            },
            handle_request_to_owner_sub_1: |me, message, cb| {
                cb(Ok(message + 1))
            },
        })?;

        // TODO - store system_ref
        // TODO - store owner_ref

        owner_ref.event_to_owner_print("bla".to_string())?;
        owner_ref.request_to_owner_sub_1(42, |me, message| {
            println!("got: {}", message);
            Ok(())
        })?;

        Ok(())
    }
}

let actor_ref = system_ref.spawn(MyActor::new(), Handler {
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

// --- demo --- ///

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

    /// spawn / manage a new actor
    pub fn spawn<'a, X: 'lt + Send + Sync, P: GhostProtocol, A: 'lt + GhostActor<'lt, P, A>>(
        &'a mut self,
        user_data: Weak<RwLock<X>>,
        mut actor: A,
    ) -> GhostResult<GhostEndpointRef<'lt, X, A, P>> {
        let (s1, r1) = crossbeam_channel::unbounded();
        let (s2, r2) = crossbeam_channel::unbounded();

        let actor = Arc::new(RwLock::new(actor));
        let weak_ref = Arc::downgrade(&actor);

        let inflator = GhostInflator {
            phantom_a: std::marker::PhantomData,
            phantom_b: std::marker::PhantomData,
            system_ref: self.clone(),
            sender: s2,
            receiver: r1,
        };

        actor
            .write()
            .expect("failed to obtain write lock")
            .actor_init(weak_ref, inflator)?;

        GhostEndpointRef::new(s1, r2, self, actor, user_data)
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
}

pub type GhostHandlerCb<'lt, T> = Box<dyn FnOnce(T) -> GhostResult<()> + 'lt + Send + Sync>;

pub type GhostResponseCb<'lt, X, T> =
    Box<dyn FnOnce(&mut X, GhostResult<T>) -> GhostResult<()> + 'lt + Send + Sync>;

#[derive(Clone, Debug)]
pub enum Fake {
    APrint(String),
    OPrint(String),
    AAdd1(i32),
    AAdd1R(Result<i32, ()>),
    OSub1(i32),
    OSub1R(Result<i32, ()>),
}

impl GhostProtocol for Fake {
    fn discriminant_list() -> &'static [GhostProtocolDiscriminant] {
        unimplemented!();
    }

    fn discriminant(&self) -> &GhostProtocolDiscriminant {
        unimplemented!();
    }
}

pub trait GhostHandler<'lt, X: 'lt + Send + Sync, P: GhostProtocol> {
    fn trigger(
        &mut self,
        user_data: &mut X,
        message: P,
        cb: Option<GhostHandlerCb<'lt, P>>,
    ) -> GhostResult<()>;
}

#[allow(clippy::complexity)]
pub struct TestActorHandler<'lt, X: 'lt + Send + Sync> {
    phantom: std::marker::PhantomData<&'lt X>,
    pub handle_event_to_actor_print: Box<dyn FnMut(&mut X, String) -> GhostResult<()> + 'lt>,
    pub handle_request_to_actor_add_1:
        Box<dyn FnMut(&mut X, i32, GhostHandlerCb<'lt, Result<i32, ()>>) -> GhostResult<()> + 'lt>,
}

impl<'lt, X: 'lt + Send + Sync> GhostHandler<'lt, X, Fake> for TestActorHandler<'lt, X> {
    fn trigger(
        &mut self,
        user_data: &mut X,
        message: Fake,
        cb: Option<GhostHandlerCb<'lt, Fake>>,
    ) -> GhostResult<()> {
        match message {
            Fake::APrint(m) => (self.handle_event_to_actor_print)(user_data, m),
            Fake::AAdd1(m) => {
                let cb = cb.unwrap();
                let cb = Box::new(move |resp| cb(Fake::AAdd1R(resp)));
                (self.handle_request_to_actor_add_1)(user_data, m, cb)
            }
            _ => panic!("bad"),
        }
    }
}

#[allow(clippy::complexity)]
pub struct TestOwnerHandler<'lt, X: 'lt + Send + Sync> {
    phantom: std::marker::PhantomData<&'lt X>,
    pub handle_event_to_owner_print: Box<dyn FnMut(&mut X, String) -> GhostResult<()> + 'lt>,
    pub handle_request_to_owner_sub_1:
        Box<dyn FnMut(&mut X, i32, GhostHandlerCb<'lt, Result<i32, ()>>) -> GhostResult<()> + 'lt>,
}

impl<'lt, X: 'lt + Send + Sync> GhostHandler<'lt, X, Fake> for TestOwnerHandler<'lt, X> {
    fn trigger(
        &mut self,
        user_data: &mut X,
        message: Fake,
        cb: Option<GhostHandlerCb<'lt, Fake>>,
    ) -> GhostResult<()> {
        match message {
            Fake::OPrint(m) => (self.handle_event_to_owner_print)(user_data, m),
            Fake::OSub1(m) => {
                let cb = cb.unwrap();
                let cb = Box::new(move |resp| cb(Fake::OSub1R(resp)));
                (self.handle_request_to_owner_sub_1)(user_data, m, cb)
            }
            _ => panic!("bad"),
        }
    }
}

pub type RequestId = String;

use std::collections::HashMap;

struct GhostEndpointRefInner<'lt, X: 'lt + Send + Sync, P: GhostProtocol> {
    pub phantom_x: std::marker::PhantomData<&'lt X>,
    pub phantom_p: std::marker::PhantomData<&'lt P>,
    receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
    req_receiver: crossbeam_channel::Receiver<(RequestId, GhostResponseCb<'lt, X, P>)>,
    callbacks: HashMap<RequestId, GhostResponseCb<'lt, X, P>>,
}

pub struct GhostEndpointRef<'lt, X: 'lt + Send + Sync, A: 'lt, P: GhostProtocol> {
    inner: Arc<RwLock<GhostEndpointRefInner<'lt, X, P>>>,
    phantom_a: std::marker::PhantomData<&'lt A>,
    sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    req_sender: crossbeam_channel::Sender<(RequestId, GhostResponseCb<'lt, X, P>)>,
    count: u64,
    a_ref: Arc<RwLock<A>>,
}

impl<'lt, X: 'lt + Send + Sync, A: 'lt, P: GhostProtocol> GhostEndpointRef<'lt, X, A, P> {
    fn new(
        sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
        receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
        system_ref: &mut GhostSystemRef<'lt>,
        a_ref: Arc<RwLock<A>>,
        user_data: Weak<RwLock<X>>,
    ) -> GhostResult<Self> {
        let (req_sender, req_receiver) = crossbeam_channel::unbounded();
        let endpoint_ref = Self {
            inner: Arc::new(RwLock::new(GhostEndpointRefInner {
                phantom_x: std::marker::PhantomData,
                phantom_p: std::marker::PhantomData,
                receiver,
                req_receiver,
                callbacks: HashMap::new(),
            })),
            phantom_a: std::marker::PhantomData,
            sender,
            req_sender,
            count: 0,
            a_ref,
        };

        let weak = Arc::downgrade(&endpoint_ref.inner);
        system_ref.enqueue_processor(Box::new(move || {
            match weak.upgrade() {
                Some(_strong_inner) => {
                    match user_data.upgrade() {
                        Some(_strong_user_data) => {
                            // TODO XXX - Hey! we have what we need to process
                            //
                            // TODO - receieve reqs / add to callbacks map
                            //
                            // TODO - receive messages - check callbacks map
                            false
                        }
                        None => false,
                    }
                }
                None => false,
            }
        }))?;

        Ok(endpoint_ref)
    }
}

impl<'lt, X: 'lt + Send + Sync, A: 'lt, P: GhostProtocol> GhostEndpoint<'lt, X, P>
    for GhostEndpointRef<'lt, X, A, P>
{
    fn send_protocol(
        &mut self,
        message: P,
        cb: Option<GhostResponseCb<'lt, X, P>>,
    ) -> GhostResult<()> {
        self.count += 1;
        match cb {
            Some(cb) => {
                let request_id = format!("req_{}", self.count);
                self.req_sender.send((request_id.clone(), cb))?;
                self.sender.send((Some(request_id), message))?;
            }
            None => {
                self.sender.send((None, message))?;
            }
        }
        Ok(())
    }
}

pub trait GhostEndpoint<'lt, X: 'lt + Send + Sync, P: GhostProtocol> {
    fn send_protocol(
        &mut self,
        message: P,
        cb: Option<GhostResponseCb<'lt, X, P>>,
    ) -> GhostResult<()>;
}

pub trait TestActorRef<'lt, X: 'lt + Send + Sync>: GhostEndpoint<'lt, X, Fake> {
    fn event_to_actor_print(&mut self, message: String) -> GhostResult<()> {
        self.send_protocol(Fake::APrint(message), None)
    }
    fn request_to_actor_add_1(
        &mut self,
        message: i32,
        cb: GhostResponseCb<'lt, X, Result<i32, ()>>,
    ) -> GhostResult<()> {
        let cb: GhostResponseCb<'lt, X, Fake> = Box::new(move |me, resp| {
            cb(
                me,
                match resp {
                    Ok(r) => match r {
                        Fake::AAdd1R(m) => Ok(m),
                        _ => panic!("bad"),
                    },
                    Err(e) => Err(e),
                },
            )
        });
        self.send_protocol(Fake::AAdd1(message), Some(cb))
    }
}

impl<'lt, X: 'lt + Send + Sync, A: 'lt> TestActorRef<'lt, X> for GhostEndpointRef<'lt, X, A, Fake> {}

pub trait TestOwnerRef<'lt, X: 'lt + Send + Sync>: GhostEndpoint<'lt, X, Fake> {
    fn event_to_owner_print(&mut self, message: String) -> GhostResult<()> {
        self.send_protocol(Fake::OPrint(message), None)
    }
    fn request_to_owner_sub_1(
        &mut self,
        message: i32,
        cb: GhostResponseCb<'lt, X, Result<i32, ()>>,
    ) -> GhostResult<()> {
        let cb: GhostResponseCb<'lt, X, Fake> = Box::new(move |me, resp| {
            cb(
                me,
                match resp {
                    Ok(r) => match r {
                        Fake::OSub1R(m) => Ok(m),
                        _ => panic!("bad"),
                    },
                    Err(e) => Err(e),
                },
            )
        });
        self.send_protocol(Fake::OSub1(message), Some(cb))
    }
}

impl<'lt, X: 'lt + Send + Sync, A: 'lt> TestOwnerRef<'lt, X> for GhostEndpointRef<'lt, X, A, Fake> {}

pub struct GhostInflator<'a, 'lt, P: GhostProtocol> {
    phantom_a: std::marker::PhantomData<&'a P>,
    phantom_b: std::marker::PhantomData<&'lt P>,
    system_ref: GhostSystemRef<'lt>,
    sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
}

impl<'a, 'lt, P: GhostProtocol> GhostInflator<'a, 'lt, P> {
    pub fn inflate<X: 'lt + Send + Sync, H: GhostHandler<'lt, X, P>>(
        mut self,
        weak_ref: Weak<RwLock<X>>,
        handler: H,
    ) -> GhostResult<(GhostSystemRef<'lt>, GhostEndpointRef<'lt, X, (), P>)> {
        let owner_ref = GhostEndpointRef::new(
            self.sender,
            self.receiver,
            &mut self.system_ref,
            Arc::new(RwLock::new(())),
            weak_ref,
        )?;
        Ok((self.system_ref, owner_ref))
    }
}

pub trait GhostActor<'lt, P: GhostProtocol, X: 'lt + Send + Sync>: Send + Sync {
    fn actor_init<'a>(
        &'a mut self,
        weak_ref: Weak<RwLock<X>>,
        inflator: GhostInflator<'a, 'lt, P>,
    ) -> GhostResult<()>;
    fn process(&mut self) -> GhostResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestActor<'lt> {
        system_ref: Option<GhostSystemRef<'lt>>,
        owner_ref: Option<GhostEndpointRef<'lt, Self, (), Fake>>,
    }

    impl<'lt> TestActor<'lt> {
        pub fn new() -> Self {
            Self {
                system_ref: None,
                owner_ref: None,
            }
        }
    }

    impl<'lt> GhostActor<'lt, Fake, TestActor<'lt>> for TestActor<'lt> {
        fn actor_init<'a>(
            &'a mut self,
            weak_ref: Weak<RwLock<TestActor<'lt>>>,
            inflator: GhostInflator<'a, 'lt, Fake>,
        ) -> GhostResult<()> {
            let (system_ref, mut owner_ref) = inflator.inflate(
                weak_ref,
                TestActorHandler {
                    phantom: std::marker::PhantomData,
                    handle_event_to_actor_print: Box::new(|_me: &mut TestActor<'lt>, message| {
                        println!("actor print: {}", message);
                        Ok(())
                    }),
                    handle_request_to_actor_add_1: Box::new(
                        |_me: &mut TestActor<'lt>, message, cb| cb(Ok(message + 1)),
                    ),
                },
            )?;
            owner_ref.event_to_owner_print("message from actor".to_string())?;
            owner_ref.request_to_owner_sub_1(
                42,
                Box::new(|_me, result| {
                    println!("got sub from owner: 42 - 1 = {:?}", result);
                    Ok(())
                }),
            )?;

            self.system_ref = Some(system_ref);
            self.owner_ref = Some(owner_ref);

            Ok(())
        }

        fn process(&mut self) -> GhostResult<()> {
            Ok(())
        }
    }

    #[test]
    fn it_can_spawn() {
        let mut system = GhostSystem::new();
        let mut system_ref = system.create_ref();

        struct MyContext {}
        let my_context = Arc::new(RwLock::new(MyContext {}));
        let my_context_weak = Arc::downgrade(&my_context);

        let mut actor_ref = system_ref
            .spawn::<MyContext, Fake, TestActor>(my_context_weak, TestActor::new())
            .unwrap();

        actor_ref
            .event_to_actor_print("zombies".to_string())
            .unwrap();
        actor_ref
            .request_to_actor_add_1(
                42,
                Box::new(|_, rsp| {
                    println!("actor got 42 + 1 = {:?}", rsp);
                    Ok(())
                }),
            )
            .unwrap();

        system.process().unwrap();
    }

    #[test]
    fn it_can_process() {
        let count = Arc::new(RwLock::new(0));
        let mut system = GhostSystem::new();

        {
            let count = count.clone();
            system
                .create_ref()
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
}
