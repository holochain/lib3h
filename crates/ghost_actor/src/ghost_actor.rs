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
    #[allow(dead_code)]
    destination: GhostProtocolDestination,
    pub(crate) phantom_x: std::marker::PhantomData<&'lt X>,
    pub(crate) phantom_p: std::marker::PhantomData<&'lt P>,
    handle_receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
    resp_sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
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
        destination: GhostProtocolDestination,
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
                destination,
                phantom_x: std::marker::PhantomData,
                phantom_p: std::marker::PhantomData,
                handle_receiver: receiver,
                resp_sender: sender.clone(),
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
        system_ref.enqueue_processor(
            0,
            Box::new(move || match weak.upgrade() {
                Some(mut strong_inner) => match user_data.upgrade() {
                    Some(mut strong_user_data) => {
                        let mut strong_inner = ghost_try_lock(&mut strong_inner);
                        let mut strong_user_data = ghost_try_lock(&mut strong_user_data);
                        while let Ok((request_id, cb)) = strong_inner.req_receiver.try_recv() {
                            strong_inner.callbacks.insert(request_id, cb);
                        }

                        while let Ok((maybe_id, message)) = strong_inner.handle_receiver.try_recv()
                        {
                            // println!("HNDL {:?} {:?} {:?}", strong_inner.destination, maybe_id, message);
                            if let GhostProtocolVariantType::Response =
                                message.discriminant().variant_type()
                            {
                                let request_id = match maybe_id {
                                    None => panic!("response with no request_id: {:?}", message),
                                    Some(request_id) => request_id,
                                };
                                match strong_inner.callbacks.remove(&request_id) {
                                    None => println!(
                                        "request_id {} not found {:?}",
                                        request_id, message
                                    ),
                                    Some(cb) => {
                                        cb(&mut strong_user_data, Ok(message)).expect("aaa");
                                    }
                                }
                            } else {
                                let cb: Option<GhostHandlerCb<'lt, P>> = match maybe_id {
                                    None => None,
                                    Some(request_id) => {
                                        let resp_sender = strong_inner.resp_sender.clone();
                                        Some(Box::new(move |message| {
                                            resp_sender.send((Some(request_id), message))?;
                                            Ok(())
                                        }))
                                    }
                                };
                                strong_inner
                                    .handler
                                    .trigger(&mut strong_user_data, message, cb)
                                    .expect("endpoint process error");
                            }
                        }

                        GhostProcessInstructions::default().set_should_continue(true)
                    }
                    None => GhostProcessInstructions::default(),
                },
                None => GhostProcessInstructions::default(),
            }),
        )?;

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
            GhostProtocolDestination::Owner,
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
