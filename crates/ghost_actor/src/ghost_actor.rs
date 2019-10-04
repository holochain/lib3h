use crate::*;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};

pub type GhostHandlerCb<'lt, T> = Box<dyn FnOnce(T) -> GhostResult<()> + 'lt + Send + Sync>;

pub type GhostResponseCb<'lt, X, T> =
    Box<dyn FnOnce(&mut X, GhostResult<T>) -> GhostResult<()> + 'lt + Send + Sync>;

pub trait GhostHandler<'lt, X: 'lt + Send + Sync, P: GhostProtocol>: Send + Sync {
    fn trigger(
        &mut self,
        user_data: &mut X,
        message: P,
        cb: Option<GhostHandlerCb<'lt, P>>,
    ) -> GhostResult<()>;
}

pub type RequestId = String;

struct GhostEndpointRefInner<
    'lt,
    X: 'lt + Send + Sync,
    P: GhostProtocol,
    H: GhostHandler<'lt, X, P>,
> {
    pub phantom_x: std::marker::PhantomData<&'lt X>,
    pub phantom_p: std::marker::PhantomData<&'lt P>,
    receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
    req_receiver: crossbeam_channel::Receiver<(RequestId, GhostResponseCb<'lt, X, P>)>,
    callbacks: HashMap<RequestId, GhostResponseCb<'lt, X, P>>,
    #[allow(dead_code)]
    handler: H,
}

pub struct GhostEndpointRef<
    'lt,
    X: 'lt + Send + Sync,
    A: 'lt,
    P: GhostProtocol,
    H: GhostHandler<'lt, X, P>,
> {
    inner: Arc<Mutex<GhostEndpointRefInner<'lt, X, P, H>>>,
    phantom_a: std::marker::PhantomData<&'lt A>,
    sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    req_sender: crossbeam_channel::Sender<(RequestId, GhostResponseCb<'lt, X, P>)>,
    count: u64,
    // just for ref counting
    _a_ref: Arc<Mutex<A>>,
}

impl<'lt, X: 'lt + Send + Sync, A: 'lt, P: GhostProtocol, H: 'lt + GhostHandler<'lt, X, P>>
    GhostEndpointRef<'lt, X, A, P, H>
{
    pub(crate) fn new(
        sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
        receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
        system_ref: &mut GhostSystemRef<'lt>,
        a_ref: Arc<Mutex<A>>,
        user_data: Weak<Mutex<X>>,
        handler: H,
    ) -> GhostResult<Self> {
        let (req_sender, req_receiver) = crossbeam_channel::unbounded();
        let endpoint_ref = Self {
            inner: Arc::new(Mutex::new(GhostEndpointRefInner {
                phantom_x: std::marker::PhantomData,
                phantom_p: std::marker::PhantomData,
                receiver,
                req_receiver,
                callbacks: HashMap::new(),
                handler,
            })),
            phantom_a: std::marker::PhantomData,
            sender,
            req_sender,
            count: 0,
            _a_ref: a_ref,
        };

        let weak = Arc::downgrade(&endpoint_ref.inner);
        system_ref.enqueue_processor(Box::new(move || match weak.upgrade() {
            Some(mut strong_inner) => match user_data.upgrade() {
                Some(mut strong_user_data) => {
                    let mut strong_inner = ghost_try_lock(&mut strong_inner);
                    #[allow(unused_variables, unused_mut)]
                    let mut strong_user_data = ghost_try_lock(&mut strong_user_data);
                    while let Ok((id, cb)) = strong_inner.req_receiver.try_recv() {
                        strong_inner.callbacks.insert(id, cb);
                    }

                    #[allow(unused_variables)]
                    while let Ok((maybe_id, message)) = strong_inner.receiver.try_recv() {
                        // TODO - actually call this thing!
                    }

                    true
                }
                None => false,
            },
            None => false,
        }))?;

        Ok(endpoint_ref)
    }
}

impl<'lt, X: 'lt + Send + Sync, A: 'lt, P: GhostProtocol, H: GhostHandler<'lt, X, P>>
    GhostEndpoint<'lt, X, P> for GhostEndpointRef<'lt, X, A, P, H>
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

pub struct GhostInflator<'a, 'lt, X: 'lt + Send + Sync, P: GhostProtocol> {
    pub(crate) phantom_a: std::marker::PhantomData<&'a P>,
    pub(crate) phantom_b: std::marker::PhantomData<&'lt P>,
    pub(crate) system_ref: GhostSystemRef<'lt>,
    pub(crate) sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    pub(crate) receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
    pub(crate) weak_ref: Weak<Mutex<X>>,
}

impl<'a, 'lt, X: 'lt + Send + Sync, P: GhostProtocol> GhostInflator<'a, 'lt, X, P> {
    pub fn inflate<H: 'lt + GhostHandler<'lt, X, P>>(
        mut self,
        handler: H,
    ) -> GhostResult<(GhostSystemRef<'lt>, GhostEndpointRef<'lt, X, (), P, H>)> {
        let owner_ref = GhostEndpointRef::new(
            self.sender,
            self.receiver,
            &mut self.system_ref,
            Arc::new(Mutex::new(())),
            self.weak_ref,
            handler,
        )?;
        Ok((self.system_ref, owner_ref))
    }
}

pub trait GhostActor<'lt, P: GhostProtocol, A: GhostActor<'lt, P, A>>: Send + Sync {
    fn actor_init<'a>(&'a mut self, inflator: GhostInflator<'a, 'lt, A, P>) -> GhostResult<()>;
    fn process(&mut self) -> GhostResult<()>;
}
