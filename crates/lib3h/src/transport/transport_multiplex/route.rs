use crate::transport::{error::*, protocol::*};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use url::Url;

use super::LocalRouteSpec;

enum RouteToParentContext {}

enum RouteToInnerContext {
    AwaitBind(
        GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
    ),
    AwaitSend(
        GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>,
    ),
}

pub struct TransportMultiplexRoute {
    #[allow(dead_code)]
    // identify ourselves as a route
    route_spec: LocalRouteSpec,
    // our parent channel endpoint
    endpoint_parent: Option<TransportActorParentEndpoint>,
    // our self channel endpoint
    endpoint_self: Detach<
        GhostContextEndpoint<
            TransportMultiplexRoute,
            RouteToParentContext,
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            TransportError,
        >,
    >,
    // ref to our inner transport
    inner_transport: Detach<
        GhostContextEndpoint<
            TransportMultiplexRoute,
            RouteToInnerContext,
            RequestToChild,
            RequestToChildResponse,
            RequestToParent,
            RequestToParentResponse,
            TransportError,
        >,
    >,
}

impl TransportMultiplexRoute {
    /// create a new TransportMultiplexRoute Instance
    pub(crate) fn new(
        route_spec: LocalRouteSpec,
        inner_transport: GhostEndpoint<
            RequestToChild,
            RequestToChildResponse,
            RequestToParent,
            RequestToParentResponse,
            TransportError,
        >,
    ) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(
            endpoint_self
                .as_context_endpoint_builder()
                .request_id_prefix("route_to_parent_")
                .build(),
        );
        let inner_transport = Detach::new(
            inner_transport
                .as_context_endpoint_builder()
                .request_id_prefix("route_to_inner_")
                .build(),
        );
        Self {
            route_spec,
            endpoint_parent,
            endpoint_self,
            inner_transport,
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
            RouteToInnerContext::AwaitBind(msg),
            RequestToChild::Bind { spec },
            Box::new(|_, context, response| {
                let msg = {
                    match context {
                        RouteToInnerContext::AwaitBind(msg) => msg,
                        _ => return Err("bad context".into()),
                    }
                };
                let response = {
                    match response {
                        GhostCallbackData::Timeout => {
                            msg.respond(Err("timeout".into()));
                            return Ok(());
                        }
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
            RouteToInnerContext::AwaitSend(msg),
            RequestToChild::SendMessage { address, payload },
            Box::new(|_, context, response| {
                let msg = {
                    match context {
                        RouteToInnerContext::AwaitSend(msg) => msg,
                        _ => return Err("bad context".into()),
                    }
                };
                let response = {
                    match response {
                        GhostCallbackData::Timeout => {
                            msg.respond(Err("timeout".into()));
                            return Ok(());
                        }
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
    > for TransportMultiplexRoute
{
    fn take_parent_endpoint(&mut self) -> Option<TransportActorParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(self))?;
        for msg in self.endpoint_self.as_mut().drain_messages() {
            self.handle_msg_from_parent(msg)?;
        }
        detach_run!(&mut self.inner_transport, |it| it.process(self))?;
        for msg in self.inner_transport.as_mut().drain_messages() {
            self.handle_msg_from_inner(msg)?;
        }
        Ok(false.into())
    }
}
