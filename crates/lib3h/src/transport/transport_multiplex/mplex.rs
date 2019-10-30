use crate::{
    error::{Lib3hError, Lib3hResult},
    gateway::protocol::*,
    new_root_span,
    transport::{error::*, protocol::*},
};
use detach::prelude::*;
use holochain_tracing::{Span, Tag};
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::Opaque, types::*, uri::Lib3hUri};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct LocalRouteSpec {
    pub space_address: SpaceHash,
    pub local_agent_id: AgentPubKey,
}

pub struct TransportMultiplex<
    G: GhostActor<
        GatewayRequestToParent,
        GatewayRequestToParentResponse,
        GatewayRequestToChild,
        GatewayRequestToChildResponse,
        Lib3hError,
    >,
> {
    // our parent channel endpoint
    endpoint_parent: Option<GatewayParentEndpoint>,
    // our self channel endpoint
    endpoint_self: Detach<GatewaySelfEndpoint<TransportMultiplex<G>>>,
    // ref to our inner gateway
    inner_gateway: Detach<GatewayParentWrapper<TransportMultiplex<G>, G>>,
    // our map of endpoints connecting us to our Routes
    route_endpoints:
        Detach<HashMap<LocalRouteSpec, TransportActorSelfEndpoint<TransportMultiplex<G>>>>,
}

impl<
        G: GhostActor<
            GatewayRequestToParent,
            GatewayRequestToParentResponse,
            GatewayRequestToChild,
            GatewayRequestToChildResponse,
            Lib3hError,
        >,
    > TransportMultiplex<G>
{
    /// create a new TransportMultiplex Instance
    pub fn new(inner_gateway: G) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(
            endpoint_self
                .as_context_endpoint_builder()
                .request_id_prefix("mplex_to_parent_")
                .build(),
        );
        let inner_gateway = Detach::new(GatewayParentWrapper::new(
            inner_gateway,
            "mplex_to_inner_gateway_",
        ));
        Self {
            endpoint_parent,
            endpoint_self,
            inner_gateway,
            route_endpoints: Detach::new(HashMap::new()),
        }
    }

    /// Return a reference to the `inner_gateway` struct field.
    pub fn inner_gateway(&self) -> &Detach<GatewayParentWrapper<TransportMultiplex<G>, G>> {
        &self.inner_gateway
    }

    /// create a route for a specific agent_id + space_address combination
    /// we are wrapping a network/node-level gateway with a machine
    /// space_address and nodeId... the space_address + agent_id parameters
    /// for this function are the higher-level notions for the AgentSpaceGateway
    pub fn create_agent_space_route(
        &mut self,
        space_address: &SpaceHash,
        local_agent_id: &AgentPubKey,
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

    /// Remove route
    pub fn remove_agent_space_route(
        &mut self,
        space_address: &SpaceHash,
        local_agent_id: &AgentPubKey,
    ) -> Option<TransportActorSelfEndpoint<TransportMultiplex<G>>> {
        let route_spec = LocalRouteSpec {
            space_address: space_address.clone(),
            local_agent_id: local_agent_id.clone(),
        };
        self.route_endpoints.remove(&route_spec)
    }

    /// The owner of this multiplex (real_engine) has received a DirectMessage
    /// these at this level are intended to be forwarded up to our routes.
    /// Collect all the un-packed info that will let us pass it back up the
    /// tree.
    pub fn received_data_for_agent_space_route(
        &mut self,
        space_address: &SpaceHash,
        local_agent_id: &AgentPubKey,
        remote_agent_id: &AgentPubKey,
        unpacked_payload: Opaque,
    ) -> Lib3hResult<()> {
        let route_spec = LocalRouteSpec {
            space_address: space_address.clone(),
            local_agent_id: local_agent_id.clone(),
        };
        let path = Lib3hUri::with_agent_id(remote_agent_id);
        match self.route_endpoints.get_mut(&route_spec) {
            None => Err(Lib3hError::new_other(&format!(
                "no such route: {:?}",
                route_spec
            ))),
            Some(ep) => {
                let mut span = new_root_span("multiplexer ReceivedData");
                span.set_tag(|| Tag::new("from", path.clone().to_string()));
                ep.publish(
                    span,
                    RequestToParent::ReceivedData {
                        uri: path,
                        payload: unpacked_payload,
                    },
                )?;
                Ok(())
            }
        }
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
        let span = msg
            .span()
            .child("request GatewayRequestToParent::Transport::ReceivedData");
        let data = msg.take_message().expect("exists");
        if let GatewayRequestToParent::Transport(RequestToParent::ReceivedData { uri, payload }) =
            data
        {
            self.handle_received_data(span, uri, payload)?;
            Ok(())
        } else {
            if msg.is_request() {
                self.endpoint_self.request(
                    span,
                    data,
                    Box::new(move |_, response| {
                        match response {
                            GhostCallbackData::Timeout(bt) => {
                                msg.respond(Err(format!("timeout: {:?}", bt).into()))?;
                                return Ok(());
                            }
                            GhostCallbackData::Response(response) => {
                                msg.respond(response)?;
                            }
                        };
                        Ok(())
                    }),
                )?;
            } else {
                self.endpoint_self.publish(span, data)?;
            }

            Ok(())
        }
    }

    /// private handler for inner transport ReceivedData events
    fn handle_received_data(
        &mut self,
        span: Span,
        uri: Lib3hUri,
        payload: Opaque,
    ) -> Lib3hResult<()> {
        // forward
        self.endpoint_self.publish(
            span,
            GatewayRequestToParent::Transport(RequestToParent::ReceivedData { uri, payload }),
        )?;
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
            RequestToChild::SendMessage { uri, payload, .. } => {
                debug!("handle_route_send to {}", uri.clone());
                self.handle_route_send_message(msg, uri, payload)
            }
        }
    }

    /// private handler for Bind requests from a route
    fn handle_route_bind(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
        spec: Lib3hUri,
    ) -> Lib3hResult<()> {
        // forward the bind to our inner_gateway
        self.inner_gateway.as_mut().request(
            msg.span()
                .child("request GatewayRequestToChild::Transport::Bind"),
            GatewayRequestToChild::Transport(RequestToChild::Bind { spec }),
            Box::new(|_, response| {
                let response = {
                    match response {
                        GhostCallbackData::Timeout(bt) => {
                            msg.respond(Err(format!("timeout: {:?}", bt).into()))?;
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
        uri: Lib3hUri,
        payload: Opaque,
    ) -> Lib3hResult<()> {
        // forward the request to our inner_gateway
        self.inner_gateway.as_mut().request(
            msg.span()
                .child("request GatewayRequestToChild::Transport::SendMessage"),
            GatewayRequestToChild::Transport(RequestToChild::create_send_message(uri, payload)),
            Box::new(|_, response| {
                let response = {
                    match response {
                        GhostCallbackData::Timeout(bt) => {
                            msg.respond(Err(format!("timeout: {:?}", bt).into()))?;
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
        if msg.is_request() {
            self.inner_gateway.as_mut().request(
                msg.span().child("handle_msg_from_parent"),
                data,
                Box::new(move |_, response| {
                    let response = {
                        match response {
                            GhostCallbackData::Timeout(bt) => {
                                msg.respond(Err(format!("timeout: {:?}", bt).into()))?;
                                return Ok(());
                            }
                            GhostCallbackData::Response(response) => response,
                        }
                    };
                    msg.respond(response)?;
                    Ok(())
                }),
            )?;
        } else {
            let orig_req = data.clone();
            self.inner_gateway.as_mut().request(
                msg.span().child("handle_msg_from_parent"),
                data,
                Box::new(move |_, response| {
                    match response {
                        GhostCallbackData::Response(Ok(response)) => {
                            trace!("mplex forward response: {:?} from {:?}", response, orig_req);
                        }
                        _ => error!("mplex bad forward: {:?} from {:?}", response, orig_req),
                    }
                    Ok(())
                }),
            )?;
        }

        Ok(())
    }
}

impl<
        G: GhostActor<
            GatewayRequestToParent,
            GatewayRequestToParentResponse,
            GatewayRequestToChild,
            GatewayRequestToChildResponse,
            Lib3hError,
        >,
    >
    GhostActor<
        GatewayRequestToParent,
        GatewayRequestToParentResponse,
        GatewayRequestToChild,
        GatewayRequestToChildResponse,
        Lib3hError,
    > for TransportMultiplex<G>
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
            let mut disconnected_endpoints = Vec::new();
            for (route_spec, endpoint) in re.iter_mut() {
                if let Err(e) = endpoint.process(self) {
                    match e.kind() {
                        lib3h_ghost_actor::ErrorKind::EndpointDisconnected => {
                            disconnected_endpoints.push(route_spec.clone());
                            continue;
                        }
                        _ => return Err(TransportError::from(e)),
                    }
                }
                for msg in endpoint.drain_messages() {
                    if let Err(e) = self.handle_msg_from_route(msg) {
                        return Err(e.into());
                    }
                }
            }
            disconnected_endpoints.iter().for_each(|e| {
                re.remove(e);
            });
            Ok(())
        })?;
        Ok(false.into())
    }
}
