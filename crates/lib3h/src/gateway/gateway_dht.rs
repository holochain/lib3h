#![allow(non_snake_case)]

use crate::{
    dht::dht_protocol::*,
    error::*,
    gateway::{protocol::*, P2pGateway},
    transport,
};
use lib3h_ghost_actor::prelude::*;

impl P2pGateway {
    /// Handle a request sent to us by our parent
    pub(crate) fn handle_dht_RequestToChild(
        &mut self,
        request: DhtRequestToChild,
        parent_msg: GatewayToChildMessage,
    ) -> Lib3hResult<()> {
        // forward to child dht
        let _ = self.inner_dht.request(
            GatewayContext::ParentRequest(parent_msg),
            request,
            Box::new(|_me, context, response| {
                let msg = {
                    match context {
                        GatewayContext::ParentRequest(parent_request) => parent_request,
                        _ => {
                            return Err(
                                format!("wanted GatewayContext::Dht, got {:?}", context).into()
                            )
                        }
                    }
                };
                let response = {
                    match response {
                        GhostCallbackData::Timeout => {
                            msg.respond(Err(Lib3hError::new_other("timeout")))?;
                            return Ok(());
                        }
                        GhostCallbackData::Response(response) => response,
                    }
                };
                // FIXME: handle it?
                // me.handle_dht_RequestToChildResponse(response)?;
                // forward back to parent
                msg.respond(Ok(GatewayRequestToChildResponse::Dht(response.unwrap())))?;
                Ok(())
            }),
        );
        // Done
        Ok(())
    }

    /// Handle a request sent to us by our child DHT
    pub(crate) fn handle_dht_RequestToParent(&mut self, mut request: DhtToParentMessage) {
        debug!(
            "({}) Serving request from child dht: {:?}",
            self.identifier, request
        );
        match request.take_message().expect("exists") {
            DhtRequestToParent::GossipTo(_data) => {
                // no-op
            }
            DhtRequestToParent::GossipUnreliablyTo(_data) => {
                // no-op
            }
            DhtRequestToParent::HoldPeerRequested(peer_data) => {
                // TODO #167 - hardcoded for MirrorDHT and thus should not appear here.
                // Connect to every peer we are requested to hold.
                info!(
                    "{} auto-connect to peer: {} ({})",
                    self.identifier, peer_data.peer_address, peer_data.peer_uri,
                );
                // Send phony SendMessage request so we connect to it
                self.child_transport_endpoint.publish(
                    transport::protocol::RequestToChild::SendMessage {
                        uri: peer_data.peer_uri,
                        payload: Vec::new(),
                    },
                );
            }
            DhtRequestToParent::PeerTimedOut(_peer_address) => {
                // TODO
            }
            // No entries in Network DHT
            DhtRequestToParent::HoldEntryRequested {
                from_peer: _,
                entry: _,
            } => {
                unreachable!();
            }
            DhtRequestToParent::EntryPruned(_) => {
                unreachable!();
            }
            DhtRequestToParent::RequestEntry(_) => {
                unreachable!();
            }
        }
    }
}
