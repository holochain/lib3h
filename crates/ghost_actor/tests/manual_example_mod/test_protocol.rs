//! The "Protocol" code generator will build the code in this file
//! when given something like:
//!
//! ```
//! ghost_protocol! {
//!     prefix(test),
//!     event_to_actor(print, String),
//!     event_to_owner(print, String),
//!     request_to_actor(add_1, i32, Result<i32, ()>),
//!     request_to_owner(sub_1, i32, Result<i32, ()>),
//! }
//! ```

use ghost_actor::prelude::*;
use holochain_tracing::*;

// -- TestProtocol -- //

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum TestProtocol {
    EventToActorPrint(String),
    EventToOwnerPrint(String),
    RequestToActorAdd1(i32),
    RequestToActorAdd1Response(Result<i32>),
    RequestToOwnerSub1(i32),
    RequestToOwnerSub1Response(Result<i32>),
}

static D_LIST: &'static [GhostProtocolDiscriminant] = &[
    GhostProtocolDiscriminant {
        id: "event_to_actor_print",
        destination: GhostProtocolDestination::Actor,
        variant_type: GhostProtocolVariantType::Event,
    },
    GhostProtocolDiscriminant {
        id: "event_to_owner_print",
        destination: GhostProtocolDestination::Owner,
        variant_type: GhostProtocolVariantType::Event,
    },
    GhostProtocolDiscriminant {
        id: "request_to_actor_add_1",
        destination: GhostProtocolDestination::Actor,
        variant_type: GhostProtocolVariantType::Request,
    },
    GhostProtocolDiscriminant {
        id: "request_to_actor_add_1_response",
        destination: GhostProtocolDestination::Owner,
        variant_type: GhostProtocolVariantType::Response,
    },
    GhostProtocolDiscriminant {
        id: "request_to_owner_sub_1",
        destination: GhostProtocolDestination::Owner,
        variant_type: GhostProtocolVariantType::Request,
    },
    GhostProtocolDiscriminant {
        id: "request_to_owner_sub_1_response",
        destination: GhostProtocolDestination::Actor,
        variant_type: GhostProtocolVariantType::Response,
    },
];

impl GhostProtocol for TestProtocol {
    fn discriminant_list() -> &'static [GhostProtocolDiscriminant] {
        D_LIST
    }

    fn discriminant(&self) -> &GhostProtocolDiscriminant {
        match self {
            TestProtocol::EventToActorPrint(_) => &D_LIST[0],
            TestProtocol::EventToOwnerPrint(_) => &D_LIST[1],
            TestProtocol::RequestToActorAdd1(_) => &D_LIST[2],
            TestProtocol::RequestToActorAdd1Response(_) => &D_LIST[3],
            TestProtocol::RequestToOwnerSub1(_) => &D_LIST[4],
            TestProtocol::RequestToOwnerSub1Response(_) => &D_LIST[5],
        }
    }
}

// -- TestActorHandler -- //

#[allow(dead_code)]
pub struct TestActorHandler<'lt, X: 'lt + Send + Sync> {
    pub handle_event_to_actor_print:
        Box<dyn FnMut(Span, &mut X, String) -> GhostResult<()> + 'lt + Send + Sync>,
    pub handle_request_to_actor_add_1: Box<
        dyn FnMut(Span, &mut X, i32, GhostHandlerCb<'lt, Result<i32>>) -> GhostResult<()>
            + 'lt
            + Send
            + Sync,
    >,
}

impl<'lt, X: 'lt + Send + Sync> GhostHandler<'lt, X, TestProtocol> for TestActorHandler<'lt, X> {
    fn trigger(
        &mut self,
        span: Span,
        user_data: &mut X,
        message: TestProtocol,
        cb: Option<GhostHandlerCb<'lt, TestProtocol>>,
    ) -> GhostResult<()> {
        match message {
            TestProtocol::EventToActorPrint(m) => {
                (self.handle_event_to_actor_print)(span, user_data, m)
            }
            TestProtocol::RequestToActorAdd1(m) => {
                let cb = cb.unwrap();
                let cb = Box::new(move |span, resp| {
                    cb(span, TestProtocol::RequestToActorAdd1Response(resp))
                });
                (self.handle_request_to_actor_add_1)(span, user_data, m, cb)
            }
            _ => panic!("bad"),
        }
    }
}

// -- TestOwnerHandler -- //

#[allow(dead_code)]
pub struct TestOwnerHandler<'lt, X: 'lt + Send + Sync> {
    pub handle_event_to_owner_print:
        Box<dyn FnMut(Span, &mut X, String) -> GhostResult<()> + 'lt + Send + Sync>,
    pub handle_request_to_owner_sub_1: Box<
        dyn FnMut(Span, &mut X, i32, GhostHandlerCb<'lt, Result<i32>>) -> GhostResult<()>
            + 'lt
            + Send
            + Sync,
    >,
}

impl<'lt, X: 'lt + Send + Sync> GhostHandler<'lt, X, TestProtocol> for TestOwnerHandler<'lt, X> {
    fn trigger(
        &mut self,
        span: Span,
        user_data: &mut X,
        message: TestProtocol,
        cb: Option<GhostHandlerCb<'lt, TestProtocol>>,
    ) -> GhostResult<()> {
        match message {
            TestProtocol::EventToOwnerPrint(m) => {
                (self.handle_event_to_owner_print)(span.child("trigger"), user_data, m)
            }
            TestProtocol::RequestToOwnerSub1(m) => {
                let cb = cb.unwrap();
                let cb = Box::new(move |span, resp| {
                    cb(span, TestProtocol::RequestToOwnerSub1Response(resp))
                });
                (self.handle_request_to_owner_sub_1)(span.child("trigger"), user_data, m, cb)
            }
            _ => panic!("bad"),
        }
    }
}

// -- TestActorRef -- //

pub trait TestActorRef<'lt, X: 'lt + Send + Sync>: GhostEndpoint<'lt, X, TestProtocol> {
    // Example helper for sending an event
    fn event_to_actor_print(
        &mut self,
        maybe_span: Option<Span>,
        message: String,
    ) -> GhostResult<()> {
        self.send_protocol(maybe_span, TestProtocol::EventToActorPrint(message), None)
    }

    // Example helper for sending a request
    fn request_to_actor_add_1(
        &mut self,
        maybe_span: Option<Span>,
        message: i32,
        response_cb: GhostResponseCb<'lt, X, Result<i32>>,
    ) -> GhostResult<()> {
        let cb: GhostResponseCb<'lt, X, TestProtocol> = Box::new(move |span, me, resp| {
            response_cb(
                span,
                me,
                match resp {
                    Ok(r) => match r {
                        TestProtocol::RequestToActorAdd1Response(m) => Ok(m),
                        _ => panic!("bad"),
                    },
                    Err(e) => Err(e),
                },
            )
        });
        self.send_protocol(
            maybe_span,
            TestProtocol::RequestToActorAdd1(message),
            Some(cb),
        )
    }
}

impl<
        'lt,
        X: 'lt + Send + Sync,
        A: 'lt,
        H: GhostHandler<'lt, X, TestProtocol>,
        S: GhostSystemRef<'lt>,
    > TestActorRef<'lt, X> for GhostEndpointFull<'lt, TestProtocol, A, X, H, S>
{
}

// -- TestOwnerRef -- //

pub trait TestOwnerRef<'lt, X: 'lt + Send + Sync>: GhostEndpoint<'lt, X, TestProtocol> {
    fn event_to_owner_print(
        &mut self,
        maybe_span: Option<Span>,
        message: String,
    ) -> GhostResult<()> {
        self.send_protocol(maybe_span, TestProtocol::EventToOwnerPrint(message), None)
    }
    fn request_to_owner_sub_1(
        &mut self,
        maybe_span: Option<Span>,
        message: i32,
        response_cb: GhostResponseCb<'lt, X, Result<i32>>,
    ) -> GhostResult<()> {
        let cb: GhostResponseCb<'lt, X, TestProtocol> = Box::new(move |span, me, resp| {
            response_cb(
                span,
                me,
                match resp {
                    Ok(r) => match r {
                        TestProtocol::RequestToOwnerSub1Response(m) => Ok(m),
                        _ => panic!("bad"),
                    },
                    Err(e) => Err(e),
                },
            )
        });
        self.send_protocol(
            maybe_span,
            TestProtocol::RequestToOwnerSub1(message),
            Some(cb),
        )
    }
}

impl<
        'lt,
        X: 'lt + Send + Sync,
        A: 'lt,
        H: GhostHandler<'lt, X, TestProtocol>,
        S: GhostSystemRef<'lt>,
    > TestOwnerRef<'lt, X> for GhostEndpointFull<'lt, TestProtocol, A, X, H, S>
{
}

// -- utility typedefs -- //

pub type TestProtocolOwnerEndpointRef<'lt, X, S> = GhostEndpointFull<'lt, TestProtocol, (), X, TestActorHandler<'lt, X>, S>;
pub type TestProtocolActorEndpointRef<'lt, D, X, S> = GhostEndpointFull<'lt, TestProtocol, D, X, TestOwnerHandler<'lt, X>, S>;
