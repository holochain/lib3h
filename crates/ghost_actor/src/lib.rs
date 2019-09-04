extern crate crossbeam_channel;
#[allow(unused_imports)]
#[macro_use]
extern crate detach;
extern crate nanoid;
#[macro_use]
extern crate shrinkwraprs;

#[derive(Shrinkwrap, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[shrinkwrap(mutable)]
pub struct WorkWasDone(pub bool);

impl From<bool> for WorkWasDone {
    fn from(b: bool) -> Self {
        WorkWasDone(b)
    }
}

impl From<WorkWasDone> for bool {
    fn from(d: WorkWasDone) -> Self {
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

mod ghost_error;
pub use ghost_error::{GhostError, GhostResult};

mod ghost_tracker;
pub use ghost_tracker::{GhostCallback, GhostCallbackData, GhostTracker};

mod ghost_channel;
pub use ghost_channel::{create_ghost_channel, GhostContextEndpoint, GhostEndpoint, GhostMessage};

mod ghost_actor;
pub use ghost_actor::{GhostActor, GhostParentWrapper, GhostParentWrapperDyn};

pub mod prelude {
    pub use super::{
        create_ghost_channel, GhostActor, GhostCallback, GhostCallbackData, GhostContextEndpoint,
        GhostEndpoint, GhostError, GhostMessage, GhostParentWrapper, GhostParentWrapperDyn,
        GhostResult, GhostTracker, WorkWasDone,
    };
}

//pub type UserData = ();

#[cfg(test)]
mod tests {
    use super::*;
    use detach::prelude::*;
    use std::any::Any;

    type FakeError = String;

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
            ResolveAddressForId(ResolveAddressForIdData),
        }

        #[derive(Debug)]
        pub enum RequestToParent {}

        #[derive(Debug)]
        pub enum RequestToParentResponse {}
    }

    struct RrDht {
        endpoint_parent: Option<
            GhostEndpoint<
                dht_protocol::RequestToChild,
                dht_protocol::RequestToChildResponse,
                dht_protocol::RequestToParent,
                dht_protocol::RequestToParentResponse,
                FakeError,
            >,
        >,
        endpoint_self: Detach<
            GhostContextEndpoint<
                RrDht,
                i8,
                dht_protocol::RequestToParent,
                dht_protocol::RequestToParentResponse,
                dht_protocol::RequestToChild,
                dht_protocol::RequestToChildResponse,
                FakeError,
            >,
        >,
    }

    impl RrDht {
        pub fn new() -> Self {
            let (endpoint_parent, endpoint_self) = create_ghost_channel();
            Self {
                endpoint_parent: Some(endpoint_parent),
                endpoint_self: Detach::new(endpoint_self.as_context_endpoint("dht_to_parent")),
            }
        }
    }

    impl
        GhostActor<
            dht_protocol::RequestToParent,
            dht_protocol::RequestToParentResponse,
            dht_protocol::RequestToChild,
            dht_protocol::RequestToChildResponse,
            FakeError,
        > for RrDht
    {
        fn as_any(&mut self) -> &mut dyn Any {
            &mut *self
        }

        fn take_parent_endpoint(
            &mut self,
        ) -> Option<
            GhostEndpoint<
                dht_protocol::RequestToChild,
                dht_protocol::RequestToChildResponse,
                dht_protocol::RequestToParent,
                dht_protocol::RequestToParentResponse,
                FakeError,
            >,
        > {
            std::mem::replace(&mut self.endpoint_parent, None)
        }

        fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
            detach_run!(&mut self.endpoint_self, |cs| { cs.process(self) })?;

            for mut msg in self.endpoint_self.as_mut().drain_messages() {
                match msg.take_message().expect("exists") {
                    dht_protocol::RequestToChild::ResolveAddressForId { id } => {
                        println!("dht got ResolveAddressForId {}", id);
                        msg.respond(Ok(
                            dht_protocol::RequestToChildResponse::ResolveAddressForId(
                                dht_protocol::ResolveAddressForIdData {
                                    address: "wss://yada".to_string(),
                                },
                            ),
                        ));
                    }
                }
            }

            Ok(false.into())
        }
    }

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
            Bind(BindResultData),
            Bootstrap,
            SendMessage,
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
        ResolveAddressForId {
            msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, FakeError>,
        },
    }

    #[derive(Debug)]
    enum RequestToParentContext {
        IncomingConnection { address: String },
    }

    struct GatewayTransport {
        endpoint_parent: Option<
            GhostEndpoint<
                RequestToChild,
                RequestToChildResponse,
                RequestToParent,
                RequestToParentResponse,
                FakeError,
            >,
        >,
        endpoint_self: Detach<
            GhostContextEndpoint<
                GatewayTransport,
                RequestToParentContext,
                RequestToParent,
                RequestToParentResponse,
                RequestToChild,
                RequestToChildResponse,
                FakeError,
            >,
        >,
        dht: Detach<
            GhostParentWrapper<
                GatewayTransport,
                GwDht,
                dht_protocol::RequestToParent,
                dht_protocol::RequestToParentResponse,
                dht_protocol::RequestToChild,
                dht_protocol::RequestToChildResponse,
                FakeError,
                RrDht,
            >,
        >,
    }

    impl GatewayTransport {
        pub fn new() -> Self {
            let (endpoint_parent, endpoint_self) = create_ghost_channel();
            let dht = Detach::new(GhostParentWrapper::new(RrDht::new(), "to_dht"));
            Self {
                endpoint_parent: Some(endpoint_parent),
                endpoint_self: Detach::new(endpoint_self.as_context_endpoint("gw_to_parent")),
                dht,
            }
        }
    }

    impl
        GhostActor<
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

        fn take_parent_endpoint(
            &mut self,
        ) -> Option<
            GhostEndpoint<
                RequestToChild,
                RequestToChildResponse,
                RequestToParent,
                RequestToParentResponse,
                FakeError,
            >,
        > {
            std::mem::replace(&mut self.endpoint_parent, None)
        }

        #[allow(irrefutable_let_patterns)]
        fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
            self.endpoint_self.as_mut().request(
                std::time::Duration::from_millis(2000),
                RequestToParentContext::IncomingConnection {
                    address: "test".to_string(),
                },
                RequestToParent::IncomingConnection {
                    address: "test".to_string(),
                },
                Box::new(|_m: &mut GatewayTransport, c, r| {
                    println!(
                        "response from parent to IncomingConnection got: {:?} with context {:?}",
                        r, c
                    );
                    Ok(())
                }),
            );
            detach_run!(&mut self.dht, |dht| { dht.process(self) })?;
            detach_run!(&mut self.endpoint_self, |endpoint_self| {
                endpoint_self.process(self)
            })?;

            for mut msg in self.endpoint_self.as_mut().drain_messages() {
                match msg.take_message().expect("exists") {
                    RequestToChild::Bind { spec: _ } => {
                        // do some internal bind
                        // we get a bound_url
                        let bound_url = "bound_url".to_string();
                        // respond to our parent
                        msg.respond(Ok(RequestToChildResponse::Bind(BindResultData {
                            bound_url: bound_url,
                        })));
                    }
                    RequestToChild::Bootstrap { address: _ } => {}
                    RequestToChild::SendMessage {
                        address,
                        payload: _,
                    } => {
                        self.dht.as_mut().request(
                            std::time::Duration::from_millis(2000),
                            GwDht::ResolveAddressForId { msg },
                            dht_protocol::RequestToChild::ResolveAddressForId { id: address },
                            Box::new(|_m:&mut GatewayTransport, context, response| {
                                let msg = {
                                    if let GwDht::ResolveAddressForId { msg } = context {
                                        msg
                                    } else {
                                        panic!("bad context type");
                                    }
                                };

                                // got a timeout error
                                if let GhostCallbackData::Timeout = response {
                                    msg.respond(Err("Timeout".into()));
                                    return Ok(());
                                }

                                let response = {
                                    if let GhostCallbackData::Response(response) = response {
                                        response
                                    } else {
                                        unimplemented!();
                                    }
                                };

                                let response = match response {
                                    Err(e) => {
                                        msg.respond(Err(e));
                                        return Ok(());
                                    }
                                    Ok(response) => response,
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

                                println!("yay? {:?}", response);

                                msg.respond(Ok(RequestToChildResponse::SendMessage));

                                Ok(())
                            }),
                        );
                    }
                }
            }
            Ok(true.into())
        }
    }

    type TransportActor = Box<
        dyn GhostActor<
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            String,
        >,
    >;

    #[test]
    fn test_ghost_example_transport() {
        // the body of this test simulates an object that contains a actor, i.e. a parent.
        // it would usually just be another ghost_actor but here we test it out explicitly
        // so first instantiate the "child" actor

        let gw = GatewayTransport::new();

        let mut t_actor: TransportActor = Box::new(gw);
        let mut t_actor_endpoint = t_actor
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint::<(), i8>("test");

        // allow the actor to run this actor always creates a simulated incoming
        // connection each time it processes
        t_actor.process().unwrap();

        let _ = t_actor_endpoint.process(&mut ());

        // now process any requests the actor may have made of us (as parent)
        for mut msg in t_actor_endpoint.drain_messages() {
            let payload = msg.take_message();
            println!("in drain_messages got: {:?}", payload);

            // we might allow or disallow connections for example
            let response = RequestToParentResponse::Allowed;
            msg.respond(Ok(response));
        }

        t_actor.process().unwrap();
        let _ = t_actor_endpoint.process(&mut ());

        // now make a request of the child,
        // to make such a request the parent would normally will also instantiate trackers so that it can
        // handle responses when they come back as callbacks.
        // here we simply watch that we got a response back as expected
        t_actor_endpoint.request(
            std::time::Duration::from_millis(2000),
            42_i8,
            RequestToChild::Bind {
                spec: "address_to_bind_to".to_string(),
            },
            Box::new(|_: &mut (), _, r| {
                println!("in callback 1, got: {:?}", r);
                Ok(())
            }),
        );

        t_actor.process().unwrap();
        let _ = t_actor_endpoint.process(&mut ());

        t_actor_endpoint.request(
            std::time::Duration::from_millis(2000),
            42_i8,
            RequestToChild::SendMessage {
                address: "agent_id_1".to_string(),
                payload: b"some content".to_vec(),
            },
            Box::new(|_: &mut (), _, r| {
                println!("in callback 2, got: {:?}", r);
                Ok(())
            }),
        );

        for _x in 0..10 {
            t_actor.process().unwrap();
            let _ = t_actor_endpoint.process(&mut ());
        }
    }
}
