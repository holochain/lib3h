extern crate nanoid;
#[macro_use]
extern crate shrinkwraprs;

#[derive(Shrinkwrap, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[shrinkwrap(mutable)]
pub struct DidWork(pub bool);

impl From<bool> for DidWork {
    fn from(b: bool) -> Self {
        DidWork(b)
    }
}

impl From<DidWork> for bool {
    fn from(d: DidWork) -> Self {
        d.0
    }
}

#[derive(Shrinkwrap, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[shrinkwrap(mutable)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn new() -> Self {
        Self::with_prefix("")
    }

    pub fn with_prefix(prefix: &str) -> Self {
        Self(format!("{}{}", prefix, nanoid::simple()))
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId(s)
    }
}

impl From<RequestId> for String {
    fn from(r: RequestId) -> Self {
        r.0
    }
}

mod ghost_tracker;
pub use ghost_tracker::{GhostCallback, GhostCallbackData, GhostTracker};

mod ghost_actor_state;
pub use ghost_actor_state::GhostActorState;

mod ghost_actor;
pub use ghost_actor::GhostActor;

pub mod prelude {
    pub use super::{GhostActor, GhostActorState, GhostCallback, GhostCallbackData, GhostTracker};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;

    #[allow(dead_code)]
    mod dht_protocol {
        #[derive(Debug)]
        pub enum RequestToChild {
            ResolveAddressForId { id: String },
        }

        #[derive(Debug)]
        pub struct ResolveAddressForIdData {
            pub address: String,
        }

        #[derive(Debug)]
        pub enum RequestToChildResponse {
            ResolveAddressForId(Result<ResolveAddressForIdData, String>),
        }

        #[derive(Debug)]
        pub enum RequestToParent {}

        #[derive(Debug)]
        pub enum RequestToParentResponse {}
    }

    struct RrDht {
        actor_state: Option<
            GhostActorState<
                i8,
                dht_protocol::RequestToParent,
                dht_protocol::RequestToParentResponse,
                dht_protocol::RequestToChildResponse,
                String,
            >,
        >,
    }

    impl RrDht {
        pub fn new() -> Self {
            Self {
                actor_state: Some(GhostActorState::new()),
            }
        }
    }

    impl
        GhostActor<
            i8,
            dht_protocol::RequestToParent,
            dht_protocol::RequestToParentResponse,
            dht_protocol::RequestToChild,
            dht_protocol::RequestToChildResponse,
            String,
        > for RrDht
    {
        fn as_any(&mut self) -> &mut dyn Any {
            &mut *self
        }

        fn get_actor_state(
            &mut self,
        ) -> &mut GhostActorState<
            i8,
            dht_protocol::RequestToParent,
            dht_protocol::RequestToParentResponse,
            dht_protocol::RequestToChildResponse,
            String,
        > {
            self.actor_state.as_mut().unwrap()
        }

        fn take_actor_state(
            &mut self,
        ) -> GhostActorState<
            i8,
            dht_protocol::RequestToParent,
            dht_protocol::RequestToParentResponse,
            dht_protocol::RequestToChildResponse,
            String,
        > {
            std::mem::replace(&mut self.actor_state, None).unwrap()
        }

        fn put_actor_state(
            &mut self,
            actor_state: GhostActorState<
                i8,
                dht_protocol::RequestToParent,
                dht_protocol::RequestToParentResponse,
                dht_protocol::RequestToChildResponse,
                String,
            >,
        ) {
            std::mem::replace(&mut self.actor_state, Some(actor_state));
        }

        // our parent is making a request of us
        fn request(
            &mut self,
            request_id: Option<RequestId>,
            request: dht_protocol::RequestToChild,
        ) {
            match request {
                dht_protocol::RequestToChild::ResolveAddressForId { id } => {
                    println!("dht got ResolveAddressForId {}", id);
                    if let Some(request_id) = request_id {
                        println!("dht ResolveAddressForId responding to parent");
                        self.get_actor_state().respond_to_parent(
                            request_id,
                            dht_protocol::RequestToChildResponse::ResolveAddressForId(Ok(
                                dht_protocol::ResolveAddressForIdData {
                                    address: "wss://yada".to_string(),
                                },
                            )),
                        );
                    }
                }
            }
        }

        fn process_concrete(&mut self) -> Result<DidWork, String> {
            Ok(true.into())
        }
    }

    type DhtActor = Box<
        dyn GhostActor<
            i8,
            dht_protocol::RequestToParent,
            dht_protocol::RequestToParentResponse,
            dht_protocol::RequestToChild,
            dht_protocol::RequestToChildResponse,
            String,
        >,
    >;

    type Url = String;
    type TransportError = String;

    #[allow(dead_code)]
    mod transport_protocol {
        use super::*;

        #[derive(Debug)]
        pub enum RequestToChild {
            Bind { spec: Url }, // wss://0.0.0.0:0 -> all network interfaces first available port
            Bootstrap { address: Url },
            SendMessage { address: Url, payload: Vec<u8> },
        }

        #[derive(Debug)]
        pub struct BindResultData {
            pub bound_url: String,
        }

        #[derive(Debug)]
        pub enum RequestToChildResponse {
            Bind(Result<BindResultData, TransportError>),
            Bootstrap(Result<(), TransportError>),
            SendMessage(Result<(), TransportError>),
        }

        #[derive(Debug)]
        pub enum RequestToParent {
            IncomingConnection { address: Url },
            ReceivedData { adress: Url, payload: Vec<u8> },
            TransportError { error: TransportError },
        }

        #[derive(Debug)]
        pub enum RequestToParentResponse {
            Allowed,    // just for testing
            Disallowed, // just for testing
        }
    }

    use transport_protocol::*;

    #[derive(Debug)]
    enum GwDht {
        ResolveAddressForId { request_id: Option<RequestId> },
    }

    #[derive(Debug)]
    enum RequestToParentContext {
        IncomingConnection { address: String },
    }

    struct GatewayTransport {
        actor_state: Option<
            GhostActorState<
                RequestToParentContext,
                RequestToParent,
                RequestToParentResponse,
                RequestToChildResponse,
                String,
            >,
        >,
        dht: DhtActor,
        dht_callbacks: Option<GhostTracker<GwDht, dht_protocol::RequestToChildResponse, String>>,
    }

    impl GatewayTransport {
        pub fn new() -> Self {
            Self {
                actor_state: Some(GhostActorState::new()),
                dht: Box::new(RrDht::new()),
                dht_callbacks: Some(GhostTracker::new("gateway_transport_dht_")),
            }
        }
    }

    impl
        GhostActor<
            RequestToParentContext,
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            String,
        > for GatewayTransport
    {
        fn as_any(&mut self) -> &mut dyn Any {
            &mut *self
        }

        fn get_actor_state(
            &mut self,
        ) -> &mut GhostActorState<
            RequestToParentContext,
            RequestToParent,
            RequestToParentResponse,
            RequestToChildResponse,
            String,
        > {
            self.actor_state.as_mut().unwrap()
        }

        fn take_actor_state(
            &mut self,
        ) -> GhostActorState<
            RequestToParentContext,
            RequestToParent,
            RequestToParentResponse,
            RequestToChildResponse,
            String,
        > {
            std::mem::replace(&mut self.actor_state, None).unwrap()
        }

        fn put_actor_state(
            &mut self,
            actor_state: GhostActorState<
                RequestToParentContext,
                RequestToParent,
                RequestToParentResponse,
                RequestToChildResponse,
                String,
            >,
        ) {
            std::mem::replace(&mut self.actor_state, Some(actor_state));
        }

        // our parent is making a request of us
        #[allow(irrefutable_let_patterns)]
        fn request(&mut self, request_id: Option<RequestId>, request: RequestToChild) {
            match request {
                RequestToChild::Bind { spec: _u } => {
                    // do some internal bind
                    // we get a bound_url
                    let bound_url = "bound_url".to_string();
                    // respond to our parent
                    if let Some(request_id) = request_id {
                        self.get_actor_state().respond_to_parent(
                            request_id,
                            RequestToChildResponse::Bind(Ok(BindResultData {
                                bound_url: bound_url,
                            })),
                        );
                    }
                }
                RequestToChild::Bootstrap { address: _ } => {}
                RequestToChild::SendMessage {
                    address,
                    payload: _,
                } => {
                    //let dht_request_id = self.dht_callbacks.bookmark(DhtUserData::RequestingAddressTranslation(request_id), Box::new(|m, user_data, response| {
                    let dht_request_id = self.dht_callbacks.as_mut().expect("exists").bookmark(
                        std::time::Duration::from_millis(2000),
                        GwDht::ResolveAddressForId { request_id },
                        Box::new(|m, context, response| {
                            let m = match m.downcast_mut::<GatewayTransport>() {
                                None => panic!("wrong type"),
                                Some(m) => m,
                            };

                            let request_id = {
                                if let GwDht::ResolveAddressForId { request_id } = context {
                                    request_id
                                } else {
                                    panic!("bad context type");
                                }
                            };

                            // got a timeout error
                            if let GhostCallbackData::Timeout = response {
                                if let Some(request_id) = request_id {
                                    m.get_actor_state().respond_to_parent(
                                        request_id,
                                        RequestToChildResponse::SendMessage(Err(
                                            "Timeout".to_string()
                                        )),
                                    );
                                }
                                return Ok(());
                            }

                            let response = {
                                if let GhostCallbackData::Response(response) = response {
                                    response
                                } else {
                                    unimplemented!();
                                }
                            };

                            let response = {
                                if let dht_protocol::RequestToChildResponse::ResolveAddressForId(
                                    response,
                                ) = response
                                {
                                    response
                                } else {
                                    panic!("aaah");
                                }
                            };

                            // got an error during dht address resolution
                            if let Err(e) = response {
                                if let Some(request_id) = request_id {
                                    m.get_actor_state().respond_to_parent(
                                        request_id,
                                        RequestToChildResponse::SendMessage(Err(e)),
                                    );
                                }
                                return Ok(());
                            }
                            let _sub_address = response.unwrap();
                            if let Some(request_id) = request_id {
                                m.get_actor_state().respond_to_parent(
                                    request_id,
                                    RequestToChildResponse::SendMessage(Ok(())),
                                );
                            }
                            Ok(())
                        }),
                    );
                    self.dht.request(
                        Some(dht_request_id),
                        dht_protocol::RequestToChild::ResolveAddressForId { id: address },
                    );
                }
            }
        }

        fn process_concrete(&mut self) -> Result<DidWork, String> {
            self.get_actor_state().send_request_to_parent(
                std::time::Duration::from_millis(2000),
                RequestToParent::IncomingConnection {
                    address: "test".to_string(),
                },
                RequestToParentContext::IncomingConnection {
                    address: "test".to_string(),
                },
                Box::new(|_m, c, r| {
                    println!(
                        "response from parent to IncomingConnection got: {:?} with context {:?}",
                        r, c
                    );
                    Ok(())
                }),
            );
            self.dht.process()?;
            for (rid, msg) in self.dht.drain_responses() {
                let mut cb = std::mem::replace(&mut self.dht_callbacks, None);
                cb.as_mut()
                    .expect("exists")
                    .handle(rid, self.as_any(), msg)?;
                std::mem::replace(&mut self.dht_callbacks, cb);
            }
            Ok(true.into())
        }
    }

    type TransportActor = Box<
        dyn GhostActor<
            RequestToParentContext,
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            String,
        >,
    >;
    use crate::RequestId;

    #[test]
    fn test_wss_transport() {
        // the body of this test simulates an object that contains a actor, i.e. a parent.
        // it would usually just be another ghost_actor but here we test it out explicitly
        // so first instantiate the "child" actor
        let mut t_actor: TransportActor = Box::new(GatewayTransport::new());

        // allow the actor to run this actor always creates a simulated incoming
        // connection each time it processes
        t_actor.process().unwrap();

        // now process any requests the actor may have made of us (as parent)
        for (rid, ev) in t_actor.drain_requests() {
            println!("in drain_requests got: {:?} {:?}", rid, ev);
            if let Some(rid) = rid {
                // we might allow or disallow connections for example
                let response = RequestToParentResponse::Allowed;
                t_actor.respond(rid, response).unwrap();
            }
        }

        // now make a request of the child,
        // to make such a request the parent would normally will also instantiate trackers so that it can
        // handle responses when they come back as callbacks.
        // here we simply watch that we got a response back as expected
        let request_id = RequestId::with_prefix("test_parent");
        t_actor.request(
            Some(request_id),
            RequestToChild::Bind {
                spec: "address_to_bind_to".to_string(),
            },
        );

        // now process the responses the actor has made to our requests
        for (rid, ev) in t_actor.drain_responses() {
            println!("in drain_responses got: {:?} {:?}", rid, ev);
        }

        let request_id = RequestId::with_prefix("test_parent");
        t_actor.request(
            Some(request_id),
            RequestToChild::SendMessage {
                address: "agent_id_1".to_string(),
                payload: b"some content".to_vec(),
            },
        );

        for _x in 0..10 {
            t_actor.process().unwrap();
            for (rid, ev) in t_actor.drain_responses() {
                println!("in drain_responses got: {:?} {:?}", rid, ev);
            }
        }
    }
}
