#![allow(non_snake_case)]

use crate::{
    dht::dht_protocol::*,
    engine::p2p_protocol::P2pProtocol,
    error::*,
    gateway::{protocol::*, send_data_types::*, P2pGateway},
};
use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use lib3h_p2p_protocol::p2p::P2pMessage;

impl P2pGateway {
    /// Handle a request sent to us by our parent
    pub(crate) fn handle_dht_RequestToChild(
        &mut self,
        _span: Span,
        request: DhtRequestToChild,
        parent_msg: GatewayToChildMessage,
    ) -> Lib3hResult<()> {
        // TODO: which span do we actually want?
        let span_parent = parent_msg.span().child("handle_dht_RequestToChild");
        // forward to child dht
        if parent_msg.is_request() {
            self.inner_dht.request(
                span_parent,
                request,
                Box::new(|_me, response| {
                    let response = {
                        match response {
                            GhostCallbackData::Timeout(bt) => {
                                parent_msg.respond(Err(format!("timeout: {:?}", bt).into()))?;
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
            )?;
        } else {
            self.inner_dht.publish(span_parent, request)?;
        }

        Ok(())
    }

    /// Handle a request sent to us by our child DHT
    #[allow(irrefutable_let_patterns)]
    pub(crate) fn handle_dht_RequestToParent(
        &mut self,
        mut request: DhtToParentMessage,
    ) -> Lib3hResult<()> {
        let span = request.span().child("handle_dht_RequestToParent");
        let payload = request.take_message().expect("exists");
        trace!(
            "({}) Serving request from child dht: {:?}",
            self.identifier.nickname,
            payload
        );
        match payload.clone() {
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
                    self.identifier.nickname, peer_data.peer_name, peer_data.peer_location,
                );
                // Send Ping so we connect to it
                let payload =
                    P2pProtocol::CapnProtoMessage(P2pMessage::create_ping(None).into_bytes())
                        .into_bytes()
                        .into();
                let uri = peer_data.get_uri();
                self.send_with_full_low_uri(
                    SendWithFullLowUri {
                        span: span.follower("DhtRequestToParent::HoldPeerRequested"),
                        full_low_uri: uri,
                        payload,
                    },
                    Box::new(|_| Ok(())),
                )?;
            }
            DhtRequestToParent::PeerTimedOut(_peer_name) => {
                // TODO
            }
            // No entries in Network DHT
            DhtRequestToParent::HoldEntryRequested {
                from_peer_name: _,
                entry: _,
            } => {
                // no-op
            }
            DhtRequestToParent::EntryPruned(_) => {
                unreachable!();
            }
            DhtRequestToParent::RequestEntry(_) => {
                let span_request = span.child("request GatewayRequestToParent::Dht::RequestEntry");
                let span_broadcast =
                    span_request.child("request GatewayRequestToParent::Dht::BroadcastEntry");
                self.endpoint_self.request(
                    span_request,
                    GatewayRequestToParent::Dht(payload),
                    Box::new(|me, response| {
                        trace!("Received requestEntry response in Gateway");
                        let dht_response = match response {
                            GhostCallbackData::Response(Ok(
                                GatewayRequestToParentResponse::Dht(d),
                            )) => d,
                            _ => panic!("invalid response type: {:?}", response),
                        };
                        // #fullsync - received entry response after request from gossip list handling,
                        // treat it as an entry from author list handling.
                        if let DhtRequestToParentResponse::RequestEntry(entry) = dht_response {
                            me.inner_dht.publish(
                                span_broadcast,
                                DhtRequestToChild::BroadcastEntry(entry),
                            )?;
                        }
                        Ok(())
                    }),
                )?;
                return Ok(());
            }
        }
        // Forward to parent
        self.endpoint_self
            .publish(span, GatewayRequestToParent::Dht(payload))?;
        // Done
        Ok(())
    }
}
