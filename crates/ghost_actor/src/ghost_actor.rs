use crate::*;
use holochain_tracing::Span;
use std::sync::{Arc, Weak};

enum GhostEndpointToInner<'lt, X: 'lt + Send + Sync, P: GhostProtocol> {
    IncomingRequest(P, Option<GhostResponseCb<'lt, X, P>>),
}

struct GhostEndpointRefInner<
    'lt,
    X: 'lt + Send + Sync,
    P: GhostProtocol,
    H: GhostHandler<'lt, X, P>,
> {
    weak_user_data: Weak<GhostMutex<X>>,
    sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    handle_receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
    recv_inner: crossbeam_channel::Receiver<GhostEndpointToInner<'lt, X, P>>,
    pending_callbacks: GhostTracker<'lt, X, P>,
    handler: H,
}

impl<'lt, X: 'lt + Send + Sync, P: GhostProtocol, H: GhostHandler<'lt, X, P>>
    GhostEndpointRefInner<'lt, X, P, H>
{
    fn priv_process(&mut self, user_data: &mut X) -> GhostResult<()> {
        if self.priv_process_inner()? {
            // we got new user data, we need to abort the current loop
            // so that we use the new user_data next process() call
            return Ok(());
        }
        self.priv_process_handle_requests(user_data)?;
        Ok(())
    }

    fn priv_process_inner(&mut self) -> GhostResult<bool> {
        while let Ok(inner_msg) = self.recv_inner.try_recv() {
            match inner_msg {
                GhostEndpointToInner::IncomingRequest(message, maybe_cb) => {
                    self.priv_process_incoming_request(message, maybe_cb)?;
                }
            }
        }
        Ok(false)
    }

    fn priv_process_incoming_request(
        &mut self,
        message: P,
        maybe_cb: Option<GhostResponseCb<'lt, X, P>>,
    ) -> GhostResult<()> {
        match maybe_cb {
            Some(cb) => {
                let request_id = self.pending_callbacks.bookmark(Span::fixme(), cb)?;

                self.sender.send((Some(request_id), message))?;
            }
            None => self.sender.send((None, message))?,
        }
        Ok(())
    }

    fn priv_process_handle_requests(&mut self, user_data: &mut X) -> GhostResult<()> {
        while let Ok((maybe_id, message)) = self.handle_receiver.try_recv() {
            if let GhostProtocolVariantType::Response = message.discriminant().variant_type() {
                let request_id = match maybe_id {
                    None => panic!("response with no request_id: {:?}", message),
                    Some(request_id) => request_id,
                };
                self.pending_callbacks.handle(request_id, message)?;
            } else {
                let cb: Option<GhostHandlerCb<'lt, P>> = match maybe_id {
                    None => None,
                    Some(request_id) => {
                        let resp_sender = self.sender.clone();
                        Some(Box::new(move |message| {
                            resp_sender.send((Some(request_id), message))?;
                            Ok(())
                        }))
                    }
                };
                self.handler.trigger(user_data, message, cb)?;
            }
        }
        Ok(())
    }
}

pub struct GhostEndpointRef<
    'lt,
    X: 'lt + Send + Sync,
    A: 'lt,
    P: GhostProtocol,
    H: GhostHandler<'lt, X, P>,
> {
    _inner: Arc<GhostMutex<GhostEndpointRefInner<'lt, X, P, H>>>,
    send_inner: crossbeam_channel::Sender<GhostEndpointToInner<'lt, X, P>>,
    a_ref: Arc<GhostMutex<A>>,
}

type GhostEndpointRefFinalizeCb<'lt, X> =
    Box<dyn FnOnce(Weak<GhostMutex<X>>) -> GhostResult<()> + 'lt>;

impl<'lt, X: 'lt + Send + Sync, A: 'lt, P: GhostProtocol, H: 'lt + GhostHandler<'lt, X, P>>
    GhostEndpointRef<'lt, X, A, P, H>
{
    fn new_partial(
        sys_ref: &mut GhostSystemRef<'lt>,
        sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
        receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
        a_ref: Arc<GhostMutex<A>>,
        handler: H,
    ) -> GhostResult<(Self, GhostEndpointRefFinalizeCb<'lt, X>)> {
        let (send_inner, recv_inner) = crossbeam_channel::unbounded();
        let inner = Arc::new(GhostMutex::new(GhostEndpointRefInner {
            weak_user_data: Weak::new(),
            sender,
            handle_receiver: receiver,
            recv_inner,
            pending_callbacks: GhostTracker::new(sys_ref.clone(), Weak::new()),
            handler,
        }));

        let weak_inner = Arc::downgrade(&inner);
        let finalize_cb =
            Box::new(
                move |user_data: Weak<GhostMutex<X>>| match weak_inner.upgrade() {
                    Some(strong_inner) => {
                        let mut strong_inner = strong_inner.lock();
                        strong_inner.weak_user_data = user_data.clone();
                        strong_inner.pending_callbacks.set_user_data(user_data)?;
                        strong_inner.pending_callbacks.periodic_task(
                            0,
                            Box::new(move |user_data| match weak_inner.upgrade() {
                                Some(strong_inner) => {
                                    let mut strong_inner = strong_inner.lock();
                                    strong_inner.priv_process(user_data)?;
                                    Ok(GhostProcessInstructions::default()
                                        .set_should_continue(true))
                                }
                                None => Ok(GhostProcessInstructions::default()),
                            }),
                        )?;
                        Ok(())
                    }
                    None => Ok(()),
                },
            );

        Ok((
            Self {
                _inner: inner,
                send_inner,
                a_ref,
            },
            finalize_cb,
        ))
    }

    pub fn as_mut(&mut self) -> GhostMutexGuard<'_, A> {
        self.a_ref.lock()
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
        self.send_inner
            .send(GhostEndpointToInner::IncomingRequest(message, cb))?;
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

pub trait GhostActor<'lt, P: GhostProtocol, A: GhostActor<'lt, P, A>>: Send + Sync {
    fn process(&mut self) -> GhostResult<()>;
}

pub struct GhostInflator<'lt, P: GhostProtocol, A: 'lt + GhostActor<'lt, P, A>> {
    finalize: Arc<GhostMutex<Option<GhostEndpointRefFinalizeCb<'lt, A>>>>,
    sys_ref: GhostSystemRef<'lt>,
    sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
}

impl<'lt, P: GhostProtocol, A: 'lt + GhostActor<'lt, P, A>> GhostInflator<'lt, P, A> {
    pub fn inflate<H: 'lt + GhostHandler<'lt, A, P>>(
        mut self,
        handler: H,
    ) -> GhostResult<GhostEndpointRef<'lt, A, (), P, H>> {
        let (owner_ref, finalize) = GhostEndpointRef::new_partial(
            &mut self.sys_ref,
            self.sender,
            self.receiver,
            // this is the deref/refcount object... but on the actor side
            // we don't give actors direct access to their owners,
            // and we certainly don't refcount them ;p
            Arc::new(GhostMutex::new(())),
            handler,
        )?;
        std::mem::replace(&mut *self.finalize.lock(), Some(finalize));

        Ok(owner_ref)
    }
}

pub type GhostActorSpawnCb<'lt, A, P> =
    Box<dyn FnOnce(GhostInflator<'lt, P, A>) -> GhostResult<A> + 'lt>;

pub fn ghost_actor_spawn<
    'lt,
    X: 'lt + Send + Sync,
    P: GhostProtocol,
    A: 'lt + GhostActor<'lt, P, A>,
    H: 'lt + GhostHandler<'lt, X, P>,
>(
    mut sys_ref: GhostSystemRef<'lt>,
    user_data: Weak<GhostMutex<X>>,
    spawn_cb: GhostActorSpawnCb<'lt, A, P>,
    handler: H,
) -> GhostResult<GhostEndpointRef<'lt, X, A, P, H>> {
    let (s1, r1) = crossbeam_channel::unbounded();
    let (s2, r2) = crossbeam_channel::unbounded();

    let finalize: Arc<GhostMutex<Option<GhostEndpointRefFinalizeCb<'lt, A>>>> =
        Arc::new(GhostMutex::new(None));

    let inflator: GhostInflator<'lt, P, A> = GhostInflator {
        finalize: finalize.clone(),
        sys_ref: sys_ref.clone(),
        sender: s2,
        receiver: r1,
    };

    let strong_actor = Arc::new(GhostMutex::new(spawn_cb(inflator)?));
    let weak_actor = Arc::downgrade(&strong_actor);

    let finalize = std::mem::replace(&mut *finalize.lock(), None);

    (finalize.expect("spawn cannot initialize owner endpoint ref"))(weak_actor.clone())?;

    sys_ref.enqueue_processor(
        0,
        Box::new(move || match weak_actor.upgrade() {
            Some(strong_actor) => {
                let mut strong_actor = strong_actor.lock();
                match strong_actor.process() {
                    Ok(()) => Ok(GhostProcessInstructions::default().set_should_continue(true)),
                    Err(e) => panic!("actor.process() error: {:?}", e),
                }
            }
            None => Ok(GhostProcessInstructions::default()),
        }),
    )?;

    let (ep, finalize) =
        GhostEndpointRef::new_partial(&mut sys_ref, s1, r2, strong_actor, handler)?;
    finalize(user_data)?;
    Ok(ep)
}
