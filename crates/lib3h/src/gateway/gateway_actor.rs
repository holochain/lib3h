use crate::{
    dht::dht_protocol::*,
    error::*,
    gateway::{protocol::*, P2pGateway},
};
use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::data_types::*;

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
            self.handle_RequestToChild(request)?;
        }

        // Process inbox from child transport & handle requests
        detach_run!(&mut self.inner_transport, |cte| { cte.process(self) })?;
        for request in self.inner_transport.drain_messages() {
            self.handle_transport_RequestToParent(request)?;
        }

        // Update this_peer cache
        self.inner_dht.request(
            Span::fixme(),
            DhtRequestToChild::RequestThisPeer,
            Box::new(|mut me, response| {
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => match response {
                            Err(e) => panic!("{:?}", e),
                            Ok(response) => response,
                        },
                    }
                };
                if let DhtRequestToChildResponse::RequestThisPeer(peer_response) = response {
                    me.this_peer = peer_response;
                } else {
                    panic!("bad response to RequestThisPeer: {:?}", response);
                }
                Ok(())
            }),
        )?;

        // Process internal dht & handle requests
        detach_run!(self.inner_dht, |dht| { dht.process(self) })?;
        for request in self.inner_dht.drain_messages() {
            self.handle_dht_RequestToParent(request)?;
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
        let span = msg.span().child("handle_RequestToChild");
        let request = msg.take_message().expect("exists");
        match request {
            GatewayRequestToChild::Transport(transport_request) => {
                // Forward to child transport
                self.handle_transport_RequestToChild(span, transport_request, msg)
            }
            GatewayRequestToChild::Dht(dht_request) => {
                // Forward to child dht
                self.handle_dht_RequestToChild(span, dht_request, msg)
            }
            GatewayRequestToChild::Bootstrap(data) => {
                self.send(span, &data.bootstrap_uri, &Opaque::new(), msg)?;
                Ok(())
            }
            GatewayRequestToChild::SendAll(_) => {
                println!("BADDBADD");
                error!("BADDBADD");
                // TODO XXX - fixme
                //unimplemented!();
                Ok(())
            }
        }
    }
}
