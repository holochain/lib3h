use crate::*;
use holochain_tracing::Span;
use std::sync::{Arc, Weak};

/// If you plant an endpoind seed "later", it will return this callback
/// allowing you to finalize it with the weak user data reference.
pub type GhostEndpointFullFinalizeCb<'lt, X> =
    Box<dyn FnOnce(Weak<GhostMutex<X>>) -> GhostResult<()> + 'lt>;

/// An incomplete GhostEndpoint. It needs to be `plant`ed to fully function
pub struct GhostEndpointSeed<'lt, P: GhostProtocol, D: 'lt, S: GhostSystemRef<'lt>> {
    send: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    recv: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
    d_ref: Arc<GhostMutex<D>>,
    sys_ref: S
 }

impl<'lt, P: GhostProtocol, D: 'lt, S:GhostSystemRef<'lt>> GhostEndpointSeed<'lt, P, D, S> {
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
        }
    }

    /// plant this seed to expand it into a usage context.
    /// This builds the "handler" for managing incoming events / requests
    /// as well as associating the user_data so callbacks have context.
    pub fn plant<X: 'lt + Send + Sync, H: 'lt + GhostHandler<'lt, X, P>>(
        self,
        weak_user_data: Weak<GhostMutex<X>>,
        handler: H,
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
        GhostEndpointFull<'lt, P, D, X, H>,
        GhostEndpointFullFinalizeCb<'lt, X>,
    )> {
        let (send_inner, recv_inner) = crossbeam_channel::unbounded();

        let inner = Arc::new(GhostMutex::new(GhostEndpointFullInner {
            sys_ref: self.sys_ref,
            weak_user_data: Weak::new(),
            send: self.send,
            recv: self.recv,
            recv_inner,
            pending_callbacks: GhostTracker::new(self.sys_ref, Weak::new()),
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
            GhostEndpointFull {
                inner,
                send_inner,
                d_ref: self.d_ref,
            },
            finalize_cb,
        ))
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
    S: GhostSystemRef<'lt>
> {
    weak_user_data: Weak<GhostMutex<X>>,
    send: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    recv: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
    recv_inner: crossbeam_channel::Receiver<GhostEndpointToInner<'lt, X, P>>,
    pending_callbacks: GhostTracker<'lt, X, P, S>,
    handler: H,
    sys_ref: S
 }

impl<'lt, P: GhostProtocol, X: 'lt + Send + Sync, H: GhostHandler<'lt, X, P>, S: GhostSystemRef<'lt> + Sync + Send>
    GhostEndpointFullInner<'lt, P, X, H, S>
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
    S: GhostSystemRef<'lt>
> {
    inner: Arc<GhostMutex<GhostEndpointFullInner<'lt, P, X, H, S>>>,
    send_inner: crossbeam_channel::Sender<GhostEndpointToInner<'lt, X, P>>,
    d_ref: Arc<GhostMutex<D>>,
}

impl<'lt, P: GhostProtocol, D: 'lt, X: 'lt + Send + Sync, H: GhostHandler<'lt, X, P>, S : GhostSystemRef<'lt>>
    GhostEndpointFull<'lt, P, D, X, H, S>
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

impl<'lt, P: GhostProtocol, D: 'lt, X: 'lt + Send + Sync, H: GhostHandler<'lt, X, P>, S: GhostSystemRef<'lt>>
    GhostEndpoint<'lt, X, P> for GhostEndpointFull<'lt, P, D, X, H, S>
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
    /// If you need to do any periodic work, you should override this
    /// default "no-op" implementation of GhostActor::process()
    fn process(&mut self) -> GhostResult<()> {
        Ok(())
    }
}

/// Slightly awkward helper class for obtaining an Endpoint for an actor's owner
pub struct GhostInflator<'lt, P: GhostProtocol, A: 'lt + GhostActor<'lt, P, A>, S: GhostSystemRef<'lt>> {
    finalize: Arc<GhostMutex<Option<GhostEndpointFullFinalizeCb<'lt, A>>>>,
    sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
    sys_ref: S 
 }

impl<'lt, P: GhostProtocol, A: 'lt + GhostActor<'lt, P, A>, S:GhostSystemRef<'lt>> GhostInflator<'lt, P, A, S> {
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
        sys_ref: sys_ref,
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

    let seed = GhostEndpointSeed::new(sys_ref, s1, r2, strong_actor);
    let ep = seed.plant(user_data, handler)?;
    Ok(ep)
}
