use crate::*;
use holochain_tracing::Span;
use std::sync::{Arc, Mutex, Weak};

struct GhostEndpointRefInner<
    'lt,
    X: 'lt + Send + Sync,
    P: GhostProtocol,
    H: GhostHandler<'lt, X, P>,
> {
    sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    handle_receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
    req_receiver: crossbeam_channel::Receiver<(P, Option<GhostResponseCb<'lt, X, P>>)>,
    pending_callbacks: GhostTracker<'lt, X, P>,
    handler: H,
}

impl<'lt, X: 'lt + Send + Sync, P: GhostProtocol, H: GhostHandler<'lt, X, P>>
    GhostEndpointRefInner<'lt, X, P, H>
{
    fn priv_process(&mut self, user_data: &mut X) -> GhostResult<()> {
        self.priv_process_incoming_requests()?;
        self.priv_process_handle_requests(user_data)?;
        Ok(())
    }

    fn priv_process_incoming_requests(&mut self) -> GhostResult<()> {
        while let Ok((message, maybe_cb)) = self.req_receiver.try_recv() {
            match maybe_cb {
                Some(cb) => {
                    let request_id = self.pending_callbacks.bookmark(Span::fixme(), cb)?;

                    self.sender.send((Some(request_id), message))?;
                }
                None => self.sender.send((None, message))?,
            }
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
    inner: Arc<Mutex<GhostEndpointRefInner<'lt, X, P, H>>>,
    req_sender: crossbeam_channel::Sender<(P, Option<GhostResponseCb<'lt, X, P>>)>,
    // just for ref counting
    _a_ref: Arc<Mutex<A>>,
}

impl<'lt, X: 'lt + Send + Sync, A: 'lt, P: GhostProtocol, H: 'lt + GhostHandler<'lt, X, P>>
    GhostEndpointRef<'lt, X, A, P, H>
{
    pub(crate) fn new(
        sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
        receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
        sys_ref: &mut GhostSystemRef<'lt>,
        a_ref: Arc<Mutex<A>>,
        user_data: Weak<Mutex<X>>,
        handler: H,
    ) -> GhostResult<Self> {
        let (req_sender, req_receiver) = crossbeam_channel::unbounded();
        let endpoint_ref = Self {
            inner: Arc::new(Mutex::new(GhostEndpointRefInner {
                sender,
                handle_receiver: receiver,
                req_receiver,
                pending_callbacks: GhostTracker::new(sys_ref.clone(), user_data.clone()),
                handler,
            })),
            req_sender,
            _a_ref: a_ref,
        };

        let weak = Arc::downgrade(&endpoint_ref.inner);
        sys_ref.enqueue_processor(
            0,
            Box::new(move || match weak.upgrade() {
                Some(mut strong_inner) => match user_data.upgrade() {
                    Some(mut strong_user_data) => {
                        let mut strong_inner = ghost_try_lock(&mut strong_inner);
                        let mut strong_user_data = ghost_try_lock(&mut strong_user_data);

                        strong_inner
                            .priv_process(&mut *strong_user_data)
                            .expect("failed endpoint ref process");

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
        self.req_sender.send((message, cb))?;
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
