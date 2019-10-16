use crate::*;
use holochain_tracing::Span;
use std::sync::Arc;

/// an actor system ref with local context
/// in general, this is passed into actor constructors
/// but cannot be used until actor_init is called
pub struct GhostActorSystem<'lt, X: 'lt + Send + Sync> {
    sys_ref: GhostSystemRef<'lt>,
    deep_user_data: DeepRef<'lt, X>,
}

/// callback for spawning a new actor
pub type GhostActorSpawnCb<'lt, A, P> = Box<
    dyn FnOnce(GhostActorSystem<'lt, A>, GhostEndpointSeed<'lt, P, ()>) -> GhostResult<A> + 'lt,
>;

impl<'lt, X: 'lt + Send + Sync> GhostActorSystem<'lt, X> {
    pub(crate) fn new(sys_ref: GhostSystemRef<'lt>, deep_user_data: DeepRef<'lt, X>) -> Self {
        Self {
            sys_ref,
            deep_user_data,
        }
    }

    /// expand an endpoint seed with local context / handling
    pub fn plant_endpoint<P: GhostProtocol, D: 'lt, H: 'lt + GhostHandler<'lt, X, P>>(
        &mut self,
        seed: GhostEndpointSeed<'lt, P, D>,
        handler: H,
    ) -> GhostResult<GhostEndpointFull<'lt, P, D, X, H>> {
        seed.priv_plant(self.deep_user_data.clone(), handler)
    }

    /// create a new sub-actor in seed form for later planting
    pub fn spawn_seed<P: GhostProtocol, A: 'lt + GhostActor<'lt, P, A>>(
        &mut self,
        spawn_cb: GhostActorSpawnCb<'lt, A, P>,
    ) -> GhostResult<GhostEndpointSeed<'lt, P, A>> {
        let (s1, r1) = crossbeam_channel::unbounded();
        let (s2, r2) = crossbeam_channel::unbounded();

        let mut sub_deep_ref = DeepRef::new();
        let sub_system = GhostActorSystem::new(self.sys_ref.clone(), sub_deep_ref.clone());

        let owner_seed =
            GhostEndpointSeed::new(self.sys_ref.clone(), s2, r1, Arc::new(GhostMutex::new(())));

        let sub_actor = Arc::new(GhostMutex::new(spawn_cb(sub_system, owner_seed)?));
        let weak_sub_actor = Arc::downgrade(&sub_actor);

        sub_deep_ref.set(weak_sub_actor.clone())?;

        // enqueue a one-time async processor to invoke actor_init
        self.sys_ref.enqueue_processor(
            0,
            Box::new(move || {
                if let Some(strong_actor) = weak_sub_actor.upgrade() {
                    strong_actor.lock().actor_init()?;
                }
                Ok(GhostProcessInstructions::default())
            }),
        )?;

        Ok(GhostEndpointSeed::new(
            self.sys_ref.clone(),
            s1,
            r2,
            sub_actor,
        ))
    }

    /// create a new full-grown sub actor
    pub fn spawn<
        P: GhostProtocol,
        A: 'lt + GhostActor<'lt, P, A>,
        H: 'lt + GhostHandler<'lt, X, P>,
    >(
        &mut self,
        spawn_cb: GhostActorSpawnCb<'lt, A, P>,
        handler: H,
    ) -> GhostResult<GhostEndpointFull<'lt, P, A, X, H>> {
        let seed = self.spawn_seed(spawn_cb)?;
        self.plant_endpoint(seed, handler)
    }
}

/// An incomplete GhostEndpoint. It needs to be `plant`ed to fully function
pub struct GhostEndpointSeed<'lt, P: GhostProtocol, D: 'lt, S: GhostSystemRef<'lt>> {
    sys_ref: S,
    send: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    recv: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
    d_ref: Arc<GhostMutex<D>>,
    _phantom: std::marker::PhantomData<&'lt S>,
}

impl<'lt, P: GhostProtocol, D: 'lt, S: GhostSystemRef<'lt>> GhostEndpointSeed<'lt, P, D, S> {
    fn new(
        sys_ref: S,
        send: crossbeam_channel::Sender<(Option<RequestId>, P)>,
        recv: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
        d_ref: Arc<GhostMutex<D>>,
    ) -> Self {
        Self {
            sys_ref,
            send,
            recv,
            d_ref,
            _phantom: std::marker::PhantomData,
        }
    }

    fn priv_plant<X: 'lt + Send + Sync, H: 'lt + GhostHandler<'lt, X, P>>(
        self,
        mut deep_user_data: DeepRef<'lt, X>,
        handler: H,
<<<<<<< HEAD
    ) -> GhostResult<GhostEndpointFull<'lt, P, D, X, H, S>> {
        let (out, finalize_cb) = self.plant_later(handler)?;
        finalize_cb(weak_user_data)?;
        Ok(out)
    }

    /// You may not yet have access to a weak reference to your user_data
    /// especially in the most normal use-case where you want the "user_data"
    /// to be a reference to the very struct you are probably constructing.
    /// `plant_later` allows you to pass in that weak user_data ref later.
    #[allow(clippy::complexity)]
    pub fn plant_later<X: 'lt + Send + Sync, H: 'lt + GhostHandler<'lt, X, P>>(
        self,
        handler: H,
    ) -> GhostResult<(
        GhostEndpointFull<'lt, P, D, X, H, S>,
        GhostEndpointFullFinalizeCb<'lt, X>,
    )> {
=======
    ) -> GhostResult<GhostEndpointFull<'lt, P, D, X, H>> {
>>>>>>> 3b5b13a57822e366f5ab2eaad105afd72d09bf5e
        let (send_inner, recv_inner) = crossbeam_channel::unbounded();
        let mut sys_ref_clone = self.sys_ref.clone();

        let inner = Arc::new(GhostMutex::new(GhostEndpointFullInner {
            sys_ref: self.sys_ref.clone(),
            send: self.send,
            recv: self.recv,
            recv_inner,
<<<<<<< HEAD
            pending_callbacks: GhostTracker::new(self.sys_ref.clone(), Weak::new()),
=======
            pending_callbacks: GhostTracker::new(self.sys_ref, deep_user_data.clone())?,
>>>>>>> 3b5b13a57822e366f5ab2eaad105afd72d09bf5e
            handler,
        }));

        let weak_inner = Arc::downgrade(&inner);
        deep_user_data.push_cb(Box::new(move |weak_user_data| {
            let weak_inner_clone = weak_inner.clone();
            if let None = weak_inner.upgrade() {
                // we don't exist anymore, let this callback get dropped
                return Ok(false);
            }
            sys_ref_clone.enqueue_processor(
                0,
                Box::new(move || match weak_inner_clone.upgrade() {
                    Some(strong_inner) => {
                        let mut strong_inner = strong_inner.lock();
                        match weak_user_data.upgrade() {
                            Some(strong_user_data) => {
                                let mut strong_user_data = strong_user_data.lock();
                                strong_inner.priv_process(&mut *strong_user_data)?;
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

        Ok(GhostEndpointFull {
            inner,
            send_inner,
            d_ref: self.d_ref,
        })
    }
}

enum GhostEndpointToInner<'lt, X: 'lt + Send + Sync, P: GhostProtocol> {
    IncomingRequest(P, Option<GhostResponseCb<'lt, X, P>>),
}

struct GhostEndpointFullInner<
    'lt,
    P: GhostProtocol,
    X: 'lt + Send + Sync,
    H: GhostHandler<'lt, X, P>,
    S: GhostSystemRef<'lt>,
> {
<<<<<<< HEAD
    weak_user_data: Weak<GhostMutex<X>>,
=======
    sys_ref: GhostSystemRef<'lt>,
>>>>>>> 3b5b13a57822e366f5ab2eaad105afd72d09bf5e
    send: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    recv: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
    recv_inner: crossbeam_channel::Receiver<GhostEndpointToInner<'lt, X, P>>,
    pending_callbacks: GhostTracker<'lt, X, P, S>,
    handler: H,
    sys_ref: S,
}

impl<
        'lt,
        P: GhostProtocol,
        X: 'lt + Send + Sync,
        H: GhostHandler<'lt, X, P>,
        S: 'lt + GhostSystemRef<'lt>,
    > GhostEndpointFullInner<'lt, P, X, H, S>
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

                self.send.send((Some(request_id), message))?;
            }
            None => self.send.send((None, message))?,
        }
        Ok(())
    }

    fn priv_process_handle_requests(&mut self, user_data: &mut X) -> GhostResult<()> {
        while let Ok((maybe_id, message)) = self.recv.try_recv() {
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
                        let resp_sender = self.send.clone();
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

/// a full `plant`ed GhostEndpoint. Used to interact with the remote end.
pub struct GhostEndpointFull<
    'lt,
    P: GhostProtocol,
    D: 'lt,
    X: 'lt + Send + Sync,
    H: GhostHandler<'lt, X, P>,
    S: GhostSystemRef<'lt>,
> {
    inner: Arc<GhostMutex<GhostEndpointFullInner<'lt, P, X, H, S>>>,
    send_inner: crossbeam_channel::Sender<GhostEndpointToInner<'lt, X, P>>,
    d_ref: Arc<GhostMutex<D>>,
}

impl<
        'lt,
        P: GhostProtocol,
        D: 'lt,
        X: 'lt + Send + Sync,
        H: GhostHandler<'lt, X, P>,
        S: GhostSystemRef<'lt>,
    > GhostEndpointFull<'lt, P, D, X, H, S>
{
    /// Sometimes you might need to invoke some functions on and endpoint
    /// before passing that endpoint off to another class. If you were invoking
    /// functions, it needed to be `plant`ed in your context. But you don't
    /// want to persist that context where you are sending it.
    /// `regress` lets you return this endpoint to seed form, so it can later
    /// be `plant`ed in a different context / with a different handler.
    pub fn regress(mut self) -> Result<GhostEndpointSeed<'lt, P, D, S>, Self> {
        // unwrapping Arc-s is weird...
        // if there is an error, put ourself back together and return
        let inner = match Arc::try_unwrap(self.inner) {
            Ok(inner) => inner,
            Err(inner) => {
                self.inner = inner;
                return Err(self);
            }
        }
        .into_inner();
        // TODO - do we want to panic! if there are pending_callbacks?
        Ok(GhostEndpointSeed::new(
            inner.sys_ref,
            inner.send,
            inner.recv,
            self.d_ref,
        ))
    }

    pub fn as_mut(&mut self) -> GhostMutexGuard<'_, D> {
        self.d_ref.lock()
    }
}

impl<
        'lt,
        P: GhostProtocol,
        D: 'lt,
        X: 'lt + Send + Sync,
        H: GhostHandler<'lt, X, P>,
        S: GhostSystemRef<'lt>,
    > GhostEndpoint<'lt, X, P> for GhostEndpointFull<'lt, P, D, X, H, S>
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

/// A GhostEndpointFull needs a raw send_protocol message.
/// But, you will most often not use this, you should use the code-generated
/// helper functions.
pub trait GhostEndpoint<'lt, X: 'lt + Send + Sync, P: GhostProtocol> {
    fn send_protocol(
        &mut self,
        message: P,
        cb: Option<GhostResponseCb<'lt, X, P>>,
    ) -> GhostResult<()>;
}

/// Describes an actor that can be used within the "Ghost" actor system
pub trait GhostActor<'lt, P: GhostProtocol, A: GhostActor<'lt, P, A>>: Send + Sync {
    /// GhostActors may not be fully usable in their constructors.
    /// While they can create EndpointFull instances, those instances don't
    /// yet have the reference context for invoking callbacks.
    /// when `actor_init` is invoked on a ghost actor, it is fully usable.
    /// You may want to trigger requests / set up processing within the
    /// this `actor_init` function invocation.
    fn actor_init(&mut self) -> GhostResult<()> {
        Ok(())
    }

    /// If you need to do any periodic work, you should override this
    /// default "no-op" implementation of GhostActor::process()
    fn process(&mut self) -> GhostResult<()> {
        Ok(())
    }
}
<<<<<<< HEAD

/// Slightly awkward helper class for obtaining an Endpoint for an actor's owner
pub struct GhostInflator<
    'lt,
    P: GhostProtocol,
    A: 'lt + GhostActor<'lt, P, A>,
    S: GhostSystemRef<'lt>,
> {
    finalize: Arc<GhostMutex<Option<GhostEndpointFullFinalizeCb<'lt, A>>>>,
    sys_ref: S,
    sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
}

impl<'lt, P: GhostProtocol, A: 'lt + GhostActor<'lt, P, A>, S: 'lt + GhostSystemRef<'lt>>
    GhostInflator<'lt, P, A, S>
{
    /// call this to get the `plant`ed full owner endpoint
    pub fn inflate<H: 'lt + GhostHandler<'lt, A, P>>(
        self,
        handler: H,
    ) -> GhostResult<GhostEndpointFull<'lt, P, (), A, H, S>> {
        let seed = GhostEndpointSeed::new(
            self.sys_ref,
            self.sender,
            self.receiver,
            // this is the deref/refcount object... but on the actor side
            // we don't give actors direct access to their owners,
            // and we certainly don't refcount them ;p
            Arc::new(GhostMutex::new(())),
        );
        let (owner_ref, finalize) = seed.plant_later(handler)?;
        std::mem::replace(&mut *self.finalize.lock(), Some(finalize));

        Ok(owner_ref)
    }
}

/// when spawning a new actor, this callback gives access to an inflator instance
/// and should return the constructed actor instance.
pub type GhostActorSpawnCb<'lt, A, P, S> =
    Box<dyn FnOnce(GhostInflator<'lt, P, A, S>) -> GhostResult<A> + 'lt>;

/// actor instances in the "Ghost" actor system should generally be spawned
/// using this function.
pub fn ghost_actor_spawn<
    'lt,
    X: 'lt + Send + Sync,
    P: GhostProtocol,
    A: 'lt + GhostActor<'lt, P, A>,
    H: 'lt + GhostHandler<'lt, X, P>,
    S: 'lt + GhostSystemRef<'lt>,
>(
    mut sys_ref: S,
    user_data: Weak<GhostMutex<X>>,
    spawn_cb: GhostActorSpawnCb<'lt, A, P, S>,
    handler: H,
) -> GhostResult<GhostEndpointFull<'lt, P, A, X, H, S>> {
    let (s1, r1) = crossbeam_channel::unbounded();
    let (s2, r2) = crossbeam_channel::unbounded();

    let finalize: Arc<GhostMutex<Option<GhostEndpointFullFinalizeCb<'lt, A>>>> =
        Arc::new(GhostMutex::new(None));

    let inflator: GhostInflator<'lt, P, A, S> = GhostInflator {
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

    let seed = GhostEndpointSeed::new(sys_ref.clone(), s1, r2, strong_actor);
    let ep = seed.plant(user_data, handler)?;
    Ok(ep)
}
=======
>>>>>>> 3b5b13a57822e366f5ab2eaad105afd72d09bf5e
