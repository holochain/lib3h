use crate::{
    error::{Lib3hError, Lib3hResult},
    gateway::{protocol::*, P2pGateway},
    transport::{error::*, protocol::*},
};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::Opaque, Address};
use lib3h_tracing::Lib3hTrace;
use std::collections::HashMap;
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct LocalRouteSpec {
    pub space_address: Address,
    pub local_agent_id: Address,
}

pub struct TransportMultiplex {
    // our parent channel endpoint
    endpoint_parent: Option<GatewayParentEndpoint>,
    // our self channel endpoint
    endpoint_self: Detach<GatewaySelfEndpoint<TransportMultiplex, Lib3hTrace>>,
    // ref to our inner gateway
    inner_gateway: Detach<GatewayParentWrapper<TransportMultiplex, Lib3hTrace, P2pGateway>>,
    // our map of endpoints connecting us to our Routes
    route_endpoints:
        Detach<HashMap<LocalRouteSpec, TransportActorSelfEndpoint<TransportMultiplex, Lib3hTrace>>>,
}

impl TransportMultiplex {
    /// create a new TransportMultiplex Instance
    pub fn new(inner_gateway: P2pGateway) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(
            endpoint_self
                .as_context_endpoint_builder()
                .request_id_prefix("mplex_to_parent_")
                .build(),
        );
        let inner_gateway =
            Detach::new(GatewayParentWrapper::new(inner_gateway, "mplex_to_inner_"));
        Self {
            endpoint_parent,
            endpoint_self,
            inner_gateway,
            route_endpoints: Detach::new(HashMap::new()),
        }
    }

    /// create a route for a specific agent_id + space_address combination
    /// we are wrapping a network/machine-level gateway with a machine
    /// space_address and machineId... the space_address + agent_id parameters
    /// for this function are the higher-level notions for the AgentSpaceGateway
    pub fn create_agent_space_route(
        &mut self,
        space_address: &Address,
        local_agent_id: &Address,
    ) -> TransportActorParentEndpoint {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_self = endpoint_self
            .as_context_endpoint_builder()
            .request_id_prefix("mplex_to_route_")
            .build();

        let route_spec = LocalRouteSpec {
            space_address: space_address.clone(),
            local_agent_id: local_agent_id.clone(),
        };

        if self
            .route_endpoints
            .insert(route_spec.clone(), endpoint_self)
            .is_some()
        {
            panic!("create_agent_space_route can only be called ONCE!");
        }

        endpoint_parent
    }

    /// The owner of this multiplex (real_engine) has received a DirectMessage
    /// these at this level are intended to be forwarded up to our routes.
    /// Collect all the un-packed info that will let us pass it back up the
    /// tree.
    pub fn received_data_for_agent_space_route(
        &mut self,
        space_address: &Address,
        local_agent_id: &Address,
        remote_agent_id: &Address,
        remote_machine_id: &Address,
        unpacked_payload: Opaque,
    ) -> Lib3hResult<()> {
        let route_spec = LocalRouteSpec {
            space_address: space_address.clone(),
            local_agent_id: local_agent_id.clone(),
        };
        let path = Url::parse(&format!(
            "transportId:{}?a={}",
            remote_machine_id, remote_agent_id
        ))
        .expect("can parse url");
        match self.route_endpoints.get_mut(&route_spec) {
            None => panic!("no such route"),
            Some(ep) => {
                ep.publish(RequestToParent::ReceivedData {
                    uri: path,
                    payload: unpacked_payload,
                })?;
            }
        }
        Ok(())
    }

    /// private dispatcher for messages from our inner transport
    fn handle_msg_from_inner(
        &mut self,
        mut msg: GhostMessage<
            GatewayRequestToParent,
            GatewayRequestToChild,
            GatewayRequestToParentResponse,
            Lib3hError,
        >,
    ) -> Lib3hResult<()> {
        let data = msg.take_message().expect("exists");
        if let GatewayRequestToParent::Transport(RequestToParent::ReceivedData { uri, payload }) =
            data
        {
            self.handle_received_data(uri, payload)?;
            Ok(())
        } else {
            self.endpoint_self.request(
                Lib3hTrace,
                data,
                Box::new(move |_, response| {
                    match response {
                        GhostCallbackData::Timeout => {
                            msg.respond(Err("timeout".into()))?;
                            return Ok(());
                        }
                        GhostCallbackData::Response(response) => {
                            msg.respond(response)?;
                        }
                    };
                    Ok(())
                }),
            )?;
            Ok(())
        }
    }

    /// private handler for inner transport ReceivedData events
    fn handle_received_data(&mut self, uri: Url, payload: Opaque) -> Lib3hResult<()> {
        // forward
        self.endpoint_self
            .publish(GatewayRequestToParent::Transport(
                RequestToParent::ReceivedData { uri, payload },
            ))?;
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
    ) -> Lib3hResult<()> {
        match msg.take_message().expect("exists") {
            RequestToChild::Bind { spec } => self.handle_route_bind(msg, spec),
            RequestToChild::SendMessage { uri, payload } => {
                self.handle_route_send_message(msg, uri, payload)
            }
        }
    }

    /// private handler for Bind requests from a route
    fn handle_route_bind(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        spec: Url,
    ) -> Lib3hResult<()> {
        // forward the bind to our inner_gateway
        self.inner_gateway.as_mut().request(
            Lib3hTrace,
            GatewayRequestToChild::Transport(RequestToChild::Bind { spec }),
            Box::new(|_, response| {
                let response = {
                    match response {
                        GhostCallbackData::Timeout => {
                            msg.respond(Err("timeout".into()))?;
                            return Ok(());
                        }
                        GhostCallbackData::Response(response) => match response {
                            Err(e) => {
                                msg.respond(Err(format!("{:?}", e).into()))?;
                                return Ok(());
                            }
                            Ok(r) => match r {
                                GatewayRequestToChildResponse::Transport(r) => Ok(r),
                                _ => {
                                    msg.respond(Err(format!("bad type: {:?}", r).into()))?;
                                    return Ok(());
                                }
                            },
                        },
                    }
                };
                msg.respond(response)?;
                Ok(())
            }),
        )?;
        Ok(())
    }

    /// private handler for SendMessage requests from a route
    fn handle_route_send_message(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        uri: Url,
        payload: Opaque,
    ) -> Lib3hResult<()> {
        // forward the request to our inner_gateway
        self.inner_gateway.as_mut().request(
            Lib3hTrace,
            GatewayRequestToChild::Transport(RequestToChild::SendMessage { uri, payload }),
            Box::new(|_, response| {
                let response = {
                    match response {
                        GhostCallbackData::Timeout => {
                            msg.respond(Err("timeout".into()))?;
                            return Ok(());
                        }
                        GhostCallbackData::Response(response) => match response {
                            Err(e) => {
                                msg.respond(Err(format!("{:?}", e).into()))?;
                                return Ok(());
                            }
                            Ok(r) => match r {
                                GatewayRequestToChildResponse::Transport(r) => Ok(r),
                                _ => {
                                    msg.respond(Err(format!("bad type: {:?}", r).into()))?;
                                    return Ok(());
                                }
                            },
                        },
                    }
                };
                msg.respond(response)?;
                Ok(())
            }),
        )?;
        Ok(())
    }

    /// private dispatcher for messages coming from our parent
    fn handle_msg_from_parent(
        &mut self,
        mut msg: GhostMessage<
            GatewayRequestToChild,
            GatewayRequestToParent,
            GatewayRequestToChildResponse,
            Lib3hError,
        >,
    ) -> Lib3hResult<()> {
        let data = msg.take_message().expect("exists");
        self.inner_gateway.as_mut().request(
            Lib3hTrace,
            data,
            Box::new(move |_, response| {
                let response = {
                    match response {
                        GhostCallbackData::Timeout => {
                            msg.respond(Err("timeout".into()))?;
                            return Ok(());
                        }
                        GhostCallbackData::Response(response) => response,
                    }
                };
                msg.respond(response)?;
                Ok(())
            }),
        )?;
        Ok(())
    }
}

impl
    GhostActor<
        GatewayRequestToParent,
        GatewayRequestToParentResponse,
        GatewayRequestToChild,
        GatewayRequestToChildResponse,
        Lib3hError,
    > for TransportMultiplex
{
    fn take_parent_endpoint(&mut self) -> Option<GatewayParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(self))?;
        for msg in self.endpoint_self.as_mut().drain_messages() {
            self.handle_msg_from_parent(msg)?;
        }
        detach_run!(&mut self.inner_gateway, |it| it.process(self))?;
        for msg in self.inner_gateway.as_mut().drain_messages() {
            self.handle_msg_from_inner(msg)?;
        }
        detach_run!(&mut self.route_endpoints, |re| {
            for (_route_spec, endpoint) in re.iter_mut() {
                if let Err(e) = endpoint.process(self) {
                    return Err(TransportError::from(e));
                }
                for msg in endpoint.drain_messages() {
                    if let Err(e) = self.handle_msg_from_route(msg) {
                        return Err(e.into());
                    }
                }
            }
            Ok(())
        })?;
        Ok(false.into())
    }
}
