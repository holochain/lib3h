use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    gateway::protocol::*,
    transport::{error::TransportError, protocol::*},
};
use lib3h_ghost_actor::prelude::*;
use url::Url;

/// GhostActor
impl GatewayActor for P2pGateway {
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
        detach_run!(&mut self.child_transport_endpoint, |cte| { cte.process(self.as_any()) })?;
        for request in self.child_transport_endpoint.as_mut().drain_messages() {
            self.handle_transport_RequestToParent(request)
                .expect("no ghost errors");
        }

        // Process internal dht & handle requests
        let _res = self.inner_dht.process(&mut self.user_data);
        for request in self.inner_dht.drain_messages() {
            self.handle_dht_RequestToParent(request)
                .expect("no ghost errors");
        }

        // Done
        Ok(WorkWasDone::from(true)) // FIXME
    }
}

/// Private internals
impl P2pGateway {
    fn handle_RequestToChild(
        &mut self,
        mut request: GatewayToChildMessage,
    ) -> Lib3hResult<()> {
        debug!(
            "({}) Serving request from parent: {:?}",
            self.identifier, request
        );
        let parent_request = request.clone();
        match request.take_message().expect("exists") {
            Transport(transport_request) => {
                // Forward to child transport
                self.handle_transport_RequestToChild(dht_request, parent_request)
            }
            Dht(dht_request) => {
                // Forward to child dht
                self.handle_dht_RequestToChild(dht_request, parent_request)
            }
            _ => Ok(()), // FIXME
        }
    }
}
