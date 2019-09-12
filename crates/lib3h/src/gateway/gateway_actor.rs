use crate::{
    error::*,
    gateway::{protocol::*, P2pGateway},
};
use lib3h_ghost_actor::prelude::*;

impl
    GhostActor<
        GatewayRequestToParent,
        GatewayRequestToParentResponse,
        GatewayRequestToChild,
        GatewayRequestToChildResponse,
        Lib3hError,
    > for P2pGateway
{
    fn take_parent_endpoint(&mut self) -> Option<GatewayParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // process inbox from parent & handle requests
        detach_run!(&mut self.endpoint_self, |es| es.process(&mut ()))?;
        for request in self.endpoint_self.as_mut().drain_messages() {
            self.handle_RequestToChild(request)
                .expect("no ghost errors");
        }

        // Process inbox from child transport & handle requests
        detach_run!(&mut self.child_transport_endpoint, |cte| {
            cte.process(self)
        })?;
        for request in self.child_transport_endpoint.drain_messages() {
            self.handle_transport_RequestToParent(request);
        }

        // Process internal dht & handle requests
        detach_run!(self.inner_dht, |dht| { dht.process(self) })?;
        for request in self.inner_dht.drain_messages() {
            self.handle_dht_RequestToParent(request);
        }

        // Done
        Ok(WorkWasDone::from(true)) // FIXME
    }
}

/// Private internals
impl P2pGateway {
    fn handle_RequestToChild(&mut self, mut msg: GatewayToChildMessage) -> Lib3hResult<()> {
        debug!(
            "({}) Serving request from parent: {:?}",
            self.identifier, msg
        );
        // let parent_request = msg.clone();
        let request = msg.take_message().expect("exists");
        match request {
            GatewayRequestToChild::Transport(transport_request) => {
                // Forward to child transport
                self.handle_transport_RequestToChild(transport_request, msg)
            }
            GatewayRequestToChild::Dht(dht_request) => {
                // Forward to child dht
                self.handle_dht_RequestToChild(dht_request, msg)
            }
            _ => Ok(()), // FIXME
        }
    }
}
