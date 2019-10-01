#![allow(non_snake_case)]

use crate::{
    dht::dht_protocol::*,
    error::*,
    gateway::{protocol::*, P2pGateway},
};
use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::data_types::Opaque;

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
                // Send phony SendMessage request so we connect to it
                self.send(
                    span.follower("DhtRequestToParent::HoldPeerRequested"),
                    peer_data.peer_name.clone().into(),
                    peer_data.peer_location,
                    Opaque::new(),
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
                unreachable!();
            }
        }
        // Forward to parent
        self.endpoint_self
            .publish(span, GatewayRequestToParent::Dht(payload))?;
        // Done
        Ok(())
    }
}
