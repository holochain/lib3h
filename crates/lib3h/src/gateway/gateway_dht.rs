#![allow(non_snake_case)]

use crate::{
    dht::dht_protocol::*,
    error::*,
    gateway::{protocol::*, P2pGateway},
    transport,
};
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::data_types::Opaque;
use lib3h_tracing::Lib3hTrace;

impl P2pGateway {
    /// Handle a request sent to us by our parent
    pub(crate) fn handle_dht_RequestToChild(
        &mut self,
        request: DhtRequestToChild,
        parent_msg: GatewayToChildMessage,
    ) -> Lib3hResult<()> {
        // forward to child dht
        self.inner_dht
            .request(
                Lib3hTrace,
                request,
                Box::new(|_me, response| {
                    let response = {
                        match response {
                            GhostCallbackData::Timeout => {
                                parent_msg.respond(Err(Lib3hError::new_other("timeout")))?;
                                return Ok(());
                            }
                            GhostCallbackData::Response(response) => response,
                        }
                    };
                    // forward back to parent
                    parent_msg
                        .respond(Ok(GatewayRequestToChildResponse::Dht(response.unwrap())))?;
                    Ok(())
                }),
            )
            .unwrap(); // FIXME unwrap
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
                let _res = self.child_transport_endpoint.publish(
                    transport::protocol::RequestToChild::SendMessage {
                        uri: peer_data.peer_uri,
                        payload: Opaque::new(),
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
