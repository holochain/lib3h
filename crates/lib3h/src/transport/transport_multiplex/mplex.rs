use crate::transport::{error::*, protocol::*};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use std::{any::Any, collections::HashMap};
use url::Url;

use super::{LocalRouteSpec, TransportMultiplexRoute};

enum MplexToParentContext {}

enum MplexToRouteContext {}

enum MplexToInnerContext {
    AwaitBind(
        GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
    ),
    AwaitSend(
        GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
    ),
}

pub struct TransportMultiplex {
    // our parent channel endpoint
    endpoint_parent: Option<TransportActorParentEndpoint>,
    // our self channel endpoint
    endpoint_self: Detach<
        GhostContextEndpoint<
            MplexToParentContext,
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            TransportError,
        >,
    >,
    // ref to our inner transport
    inner_transport: Detach<TransportActorParentWrapperDyn<MplexToInnerContext>>,
    // our map of endpoints connecting us to our Routes
    route_endpoints: Detach<
        HashMap<
            LocalRouteSpec,
            GhostContextEndpoint<
                MplexToRouteContext,
                RequestToParent,
                RequestToParentResponse,
                RequestToChild,
                RequestToChildResponse,
                TransportError,
            >,
        >,
    >,
}

impl TransportMultiplex {
    /// create a new TransportMultiplex Instance
    pub fn new(inner_transport: DynTransportActor) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(endpoint_self.as_context_endpoint("mplex_to_parent_"));
        let inner_transport =
            Detach::new(GhostParentWrapperDyn::new(inner_transport, "mplex_to_inner_"));
        Self {
            endpoint_parent,
            endpoint_self,
            inner_transport,
            route_endpoints: Detach::new(HashMap::new()),
        }
    }

    /// create a route for a specific agent_id + space_address combination
    /// we are wrapping a network/machine-level gateway with a machine
    /// space_address and machineId... the space_address + agent_id parameters
    /// for this function are the higher-level notions for the AgentSpaceGateway
    pub fn get_agent_space_route(
        &mut self,
        space_address: String,
        local_agent_id: String,
    ) -> TransportMultiplexRoute {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_self = endpoint_self.as_context_endpoint("mplex_to_route_");

        let route_spec = LocalRouteSpec {
            space_address,
            local_agent_id,
        };

        if self
            .route_endpoints
            .insert(route_spec.clone(), endpoint_self)
            .is_some()
        {
            panic!("get_agent_space_route can only be called ONCE!");
        }

        TransportMultiplexRoute::new(route_spec, endpoint_parent)
    }

    /// The owner of this multiplex (real_engine) has received a DirectMessage
    /// these at this level are intended to be forwarded up to our routes.
    /// Collect all the un-packed info that will let us pass it back up the
    /// chain.
    pub fn received_data_for_agent_space_route(
        &mut self,
        space_address: String,
        local_agent_id: String,
        remote_agent_id: String,
        remote_machine_id: String,
        unpacked_payload: Vec<u8>,
    ) {
        let route_spec = LocalRouteSpec {
            space_address,
            local_agent_id,
        };
        let address = Url::parse(&format!(
            "transportId:{}?a={}",
            remote_machine_id, remote_agent_id
        ))
        .expect("can parse url");
        match self.route_endpoints.get_mut(&route_spec) {
            None => panic!("no such route"),
            Some(ep) => {
                ep.publish(RequestToParent::ReceivedData {
                    address,
                    payload: unpacked_payload,
                });
            }
        }
    }

    /// private dispatcher for messages from our inner transport
    fn handle_msg_from_inner(
        &mut self,
        mut msg: GhostMessage<
            RequestToParent,
            RequestToChild,
            RequestToParentResponse,
            TransportError,
        >,
    ) -> TransportResult<()> {
        match msg.take_message().expect("exists") {
            RequestToParent::IncomingConnection { address } => {
                self.handle_incoming_connection(address)
            }
            RequestToParent::ReceivedData { address, payload } => {
                self.handle_received_data(address, payload)
            }
            RequestToParent::TransportError { error } => self.handle_transport_error(error),
        }
    }

    /// private handler for inner transport IncomingConnection events
    fn handle_incoming_connection(&mut self, address: Url) -> TransportResult<()> {
        // forward
        self.endpoint_self
            .publish(RequestToParent::IncomingConnection { address });
        Ok(())
    }

    /// private handler for inner transport ReceivedData events
    fn handle_received_data(&mut self, address: Url, payload: Vec<u8>) -> TransportResult<()> {
        // forward
        self.endpoint_self
            .publish(RequestToParent::ReceivedData { address, payload });
        Ok(())
    }

    /// private handler for inner transport TransportError events
    fn handle_transport_error(&mut self, error: TransportError) -> TransportResult<()> {
        // forward
        self.endpoint_self
            .publish(RequestToParent::TransportError { error });
        Ok(())
    }

    /// private dispatcher for messages coming from our parent
    fn handle_msg_from_route(
        &mut self,
        mut msg: GhostMessage<
            RequestToChild,
            RequestToParent,
            RequestToChildResponse,
            TransportError,
        >,
    ) -> TransportResult<()> {
        match msg.take_message().expect("exists") {
            RequestToChild::Bind { spec } => self.handle_route_bind(msg, spec),
            RequestToChild::SendMessage { address, payload } => {
                self.handle_route_send_message(msg, address, payload)
            }
        }
    }

    /// private handler for Bind requests from a route
    fn handle_route_bind(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        spec: Url,
    ) -> TransportResult<()> {
        // ... I have no idea what this should do : )
        unimplemented!();
    }

    /// private handler for SendMessage requests from a route
    fn handle_route_send_message(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        address: Url,
        payload: Vec<u8>,
    ) -> TransportResult<()> {
        // forward the request to our inner_transport
        self.inner_transport.as_mut().request(
            std::time::Duration::from_millis(2000),
            MplexToInnerContext::AwaitSend(msg),
            RequestToChild::SendMessage { address, payload },
            Box::new(|_, context, response| {
                let msg = {
                    match context {
                        MplexToInnerContext::AwaitSend(msg) => msg,
                        _ => panic!("bad context"),
                    }
                };
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => response,
                    }
                };
                msg.respond(response);
                Ok(())
            }),
        );
        Ok(())
    }

    /// private dispatcher for messages coming from our parent
    fn handle_msg_from_parent(
        &mut self,
        mut msg: GhostMessage<
            RequestToChild,
            RequestToParent,
            RequestToChildResponse,
            TransportError,
        >,
    ) -> TransportResult<()> {
        match msg.take_message().expect("exists") {
            RequestToChild::Bind { spec } => self.handle_bind(msg, spec),
            RequestToChild::SendMessage { address, payload } => {
                self.handle_send_message(msg, address, payload)
            }
        }
    }

    /// private handler for Bind requests from our parent
    fn handle_bind(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        spec: Url,
    ) -> TransportResult<()> {
        // forward the bind to our inner_transport
        self.inner_transport.as_mut().request(
            std::time::Duration::from_millis(2000),
            MplexToInnerContext::AwaitBind(msg),
            RequestToChild::Bind { spec },
            Box::new(|_, context, response| {
                let msg = {
                    match context {
                        MplexToInnerContext::AwaitBind(msg) => msg,
                        _ => panic!("bad context"),
                    }
                };
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => response,
                    }
                };
                msg.respond(response);
                Ok(())
            }),
        );
        Ok(())
    }

    /// private handler for SendMessage requests from our parent
    fn handle_send_message(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        address: Url,
        payload: Vec<u8>,
    ) -> TransportResult<()> {
        // forward the request to our inner_transport
        self.inner_transport.as_mut().request(
            std::time::Duration::from_millis(2000),
            MplexToInnerContext::AwaitSend(msg),
            RequestToChild::SendMessage { address, payload },
            Box::new(|_, context, response| {
                let msg = {
                    match context {
                        MplexToInnerContext::AwaitSend(msg) => msg,
                        _ => panic!("bad context"),
                    }
                };
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => response,
                    }
                };
                msg.respond(response);
                Ok(())
            }),
        );
        Ok(())
    }
}

impl
    GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        TransportError,
    > for TransportMultiplex
{
    fn as_any(&mut self) -> &mut dyn Any {
        &mut *self
    }

    fn take_parent_endpoint(&mut self) -> Option<TransportActorParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(self.as_any()))?;
        for msg in self.endpoint_self.as_mut().drain_messages() {
            self.handle_msg_from_parent(msg)?;
        }
        detach_run!(&mut self.inner_transport, |it| it.process(self.as_any()))?;
        for msg in self.inner_transport.as_mut().drain_messages() {
            self.handle_msg_from_inner(msg)?;
        }
        detach_run!(&mut self.route_endpoints, |re| {
            for (_route_spec, endpoint) in re.iter_mut() {
                if let Err(e) = endpoint.process(self.as_any()) {
                    return Err(TransportError::from(e));
                }
                for msg in endpoint.drain_messages() {
                    if let Err(e) = self.handle_msg_from_route(msg) {
                        return Err(e);
                    }
                }
            }
            Ok(())
        })?;
        Ok(false.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID_1: &'static str = "HcSCJ9G64XDKYo433rIMm57wfI8Y59Udeb4hkVvQBZdm6bgbJ5Wgs79pBGBcuzz";
    const ID_2: &'static str = "HcMCJ8HpYvB4zqic93d3R4DjkVQ4hhbbv9UrZmWXOcn3m7w4O3AIr56JRfrt96r";

    pub struct TransportMock {
        endpoint_parent: Option<TransportActorParentEndpoint>,
        endpoint_self: Detach<
            GhostContextEndpoint<
                MplexToParentContext,
                RequestToParent,
                RequestToParentResponse,
                RequestToChild,
                RequestToChildResponse,
                TransportError,
            >,
        >,
        bound_url: Url,
        mock_sender: crossbeam_channel::Sender<(Url, Vec<u8>)>,
        mock_receiver: crossbeam_channel::Receiver<(Url, Vec<u8>)>,
    }

    impl TransportMock {
        pub fn new(
            mock_sender: crossbeam_channel::Sender<(Url, Vec<u8>)>,
            mock_receiver: crossbeam_channel::Receiver<(Url, Vec<u8>)>,
        ) -> Self {
            let (endpoint_parent, endpoint_self) = create_ghost_channel();
            let endpoint_parent = Some(endpoint_parent);
            let endpoint_self = Detach::new(endpoint_self.as_context_endpoint("mock_to_parent_"));
            Self {
                endpoint_parent,
                endpoint_self,
                bound_url: Url::parse("none:").expect("can parse url"),
                mock_sender,
                mock_receiver,
            }
        }
    }

    impl
        GhostActor<
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            TransportError,
        > for TransportMock
    {
        fn as_any(&mut self) -> &mut dyn Any {
            &mut *self
        }

        fn take_parent_endpoint(&mut self) -> Option<TransportActorParentEndpoint> {
            std::mem::replace(&mut self.endpoint_parent, None)
        }

        fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
            detach_run!(&mut self.endpoint_self, |es| es.process(self.as_any()))?;
            for mut msg in self.endpoint_self.as_mut().drain_messages() {
                match msg.take_message().expect("exists") {
                    RequestToChild::Bind { mut spec } => {
                        spec.set_path("bound");
                        self.bound_url = spec.clone();
                        msg.respond(Ok(RequestToChildResponse::Bind(BindResultData {
                            bound_url: spec,
                        })));
                    }
                    RequestToChild::SendMessage { address, payload } => {
                        self.mock_sender.send((address, payload)).unwrap();
                        msg.respond(Ok(RequestToChildResponse::SendMessage));
                    }
                }
            }
            loop {
                match self.mock_receiver.try_recv() {
                    Ok((address, payload)) => {
                        // bit of a hack, just always send an incoming connection
                        // in front of all received data messages
                        self.endpoint_self
                            .publish(RequestToParent::IncomingConnection {
                                address: address.clone(),
                            });
                        self.endpoint_self
                            .publish(RequestToParent::ReceivedData { address, payload });
                    }
                    Err(_) => break,
                }
            }
            Ok(false.into())
        }
    }

    #[test]
    fn it_should_exchange_messages() {
        // set up some reference values
        let addr1 = Url::parse("test://1/bound").unwrap();
        let addr2 = Url::parse("test://2/bound").unwrap();
        let mut addr1full = addr1.clone();
        addr1full.query_pairs_mut().append_pair("a", ID_1);
        let mut addr2full = addr2.clone();
        addr2full.query_pairs_mut().append_pair("a", ID_2);

        // we need some channels into our mock inner_transports
        let (s1out, r1out) = crossbeam_channel::unbounded();
        let (s1in, r1in) = crossbeam_channel::unbounded();

        // create the first encoding transport
        let mut t1: TransportActorParentWrapperDyn<()> = GhostParentWrapperDyn::new(
            Box::new(TransportMultiplex::new(Box::new(TransportMock::new(
                s1out, r1in,
            )))),
            "test1",
        );

        // give it a bind point
        t1.request(
            std::time::Duration::from_millis(2000),
            (),
            RequestToChild::Bind {
                spec: Url::parse("test://1?a=HcSCJ9G64XDKYo433rIMm57wfI8Y59Udeb4hkVvQBZdm6bgbJ5Wgs79pBGBcuzz").expect("can parse url"),
            },
            Box::new(|_, _, response| {
                assert_eq!(
                    &format!("{:?}", response),
                    "Response(Ok(Bind(BindResultData { bound_url: \"test://1/bound?a=HcSCJ9G64XDKYo433rIMm57wfI8Y59Udeb4hkVvQBZdm6bgbJ5Wgs79pBGBcuzz\" })))"
                );
                Ok(())
            })
        );

        // allow process
        t1.process(&mut ()).unwrap();

        // we need some channels into our mock inner_transports
        let (s2out, r2out) = crossbeam_channel::unbounded();
        let (s2in, r2in) = crossbeam_channel::unbounded();

        // create the second encoding transport
        let mut t2: TransportActorParentWrapperDyn<()> = GhostParentWrapperDyn::new(
            Box::new(TransportMultiplex::new(Box::new(TransportMock::new(
                s2out, r2in,
            )))),
            "test2",
        );

        // give it a bind point
        t2.request(
            std::time::Duration::from_millis(2000),
            (),
            RequestToChild::Bind {
                spec: Url::parse("test://2?a=HcMCJ8HpYvB4zqic93d3R4DjkVQ4hhbbv9UrZmWXOcn3m7w4O3AIr56JRfrt96r").expect("can parse url"),
            },
            Box::new(|_, _, response| {
                assert_eq!(
                    &format!("{:?}", response),
                    "Response(Ok(Bind(BindResultData { bound_url: \"test://2/bound?a=HcMCJ8HpYvB4zqic93d3R4DjkVQ4hhbbv9UrZmWXOcn3m7w4O3AIr56JRfrt96r\" })))"
                );
                Ok(())
            })
        );

        // allow process
        t2.process(&mut ()).unwrap();
    }
}
