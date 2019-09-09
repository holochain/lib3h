#![allow(non_snake_case)]

use crate::{
    dht::dht_protocol::*,
    engine::p2p_protocol::P2pProtocol,
    gateway::{Gateway, P2pGateway},
    transport::error::{TransportError, TransportResult},
};
use lib3h_protocol::DidWork;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;


impl P2pGateway {
    /// Handle a request sent to us by our parent
    fn handle_dht_RequestToChild(
        &mut self,
        mut request: DhtToChildMessage,
    ) -> Lib3hResult<()> {
        // forward to child dht
        let _ = self.inner_dht.request(
            GatewayContext::Dht { parent_request: dht_request},
            transport_request,
            Box::new(|_me, context, response| {
                let msg = {
                    match context {
                        GatewayContext::Dht { parent_request } => parent_request,
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
                            msg.respond(Err("timeout".into()))?;
                            return Ok(());
                        }
                        GhostCallbackData::Response(response) => response,
                    }
                };
                // FIXME: handle it?
                // me.handle_dht_RequestToChildResponse(response)?;
                // forward back to parent
                msg.respond(response)?;
                Ok(())
            }),
        );
        // Done
        Ok(())
    }


    /// Handle a request sent to us by our child DHT
    fn handle_dht_RequestToParent(&mut self, mut request: DhtToParentMessage) {
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
                self.child_transport_endpoint
                    .publish(TransportRequestToChild::SendMessage {
                        address: peer_data.peer_uri,
                        payload: Vec::new(),
                    });
            }
            DhtRequestToParent::PeerTimedOut(peer_address) => {
                // TODO
            }
            // No entries in Network DHT
            DhtRequestToParent::HoldEntryRequested(_, _) => {
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