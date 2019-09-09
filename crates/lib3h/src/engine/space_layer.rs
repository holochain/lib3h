#![allow(non_snake_case)]

use super::RealEngineTrackerData;
use crate::{
    dht::dht_protocol::*,
    engine::{p2p_protocol::SpaceAddress, real_engine::handle_gossipTo, ChainId, RealEngine},
    gateway::protocol::*,
};
use lib3h_protocol::{
    data_types::*, error::Lib3hProtocolResult, protocol_server::Lib3hServerProtocol,
};
use std::collections::HashMap;

/// Space layer related private methods
/// Engine does not process a space gateway's Transport because it is shared with the network layer
impl RealEngine {
    /// Return list of space+this_peer for all currently joined Spaces
    pub fn get_all_spaces(&self) -> Vec<(SpaceAddress, PeerData)> {
        let mut result = Vec::new();
        for (chainId, space_gateway) in self.space_gateway_map.iter() {
            let space_address: String = chainId.0.clone().into();
            result.push((
                space_address,
                space_gateway.as_mut().get_this_peer_sync().clone(),
            ));
        }
        result
    }

    /// Return first space gateway for a specified space_address
    pub fn get_first_space_mut(
        &mut self,
        space_address: &str,
    ) -> Option<GatewayParentWrapperDyn<(), GatewayContext>> {
        for (chainId, space_gateway) in self.space_gateway_map.iter_mut() {
            let current_space_address: String = chainId.0.clone().into();
            if current_space_address == space_address {
                return Some(space_gateway.clone());
            }
        }
        None
    }

    /// Process all space gateways
    pub(crate) fn process_space_gateways(
        &mut self,
    ) -> Lib3hProtocolResult<Vec<Lib3hServerProtocol>> {
        // Process all space gateways and collect requests
        let mut outbox = Vec::new();
        let mut space_outbox_map = HashMap::new();
        for (chain_id, space_gateway) in self.space_gateway_map.iter_mut() {
            detach_run!(&mut self.space_gateway, |sg| sg.process(&mut ()))?;
            let request_list = self.space_gateway.drain_messages();
            space_outbox_map.insert(chain_id.clone(), request_list);
            // DHT magic
            let mut temp = space_gateway.as_mut().drain_dht_outbox();
            self.temp_outbox.append(&mut temp);
        }
        // Process all space gateway requests
        for (chain_id, request_list) in space_outbox_map {
            for mut request in request_list {
                let payload = request.take_message().expect("exists");
                let mut output = self.handle_space_request(&chain_id, payload)?;
                outbox.append(&mut output);
            }
        }
        // Done
        Ok(outbox)
    }

    /// Handle a GatewayRequestToParent sent to us by one of our space gateway
    fn handle_space_request(
        &mut self,
        chain_id: &ChainId,
        request: GatewayRequestToParent,
    ) -> Lib3hProtocolResult<Vec<Lib3hServerProtocol>> {
        debug!(
            "{} << handle_space_request: [{:?}] - {:?}",
            self.name, chain_id, request,
        );
        let mut outbox = Vec::new();
        let space_gateway = self
            .space_gateway_map
            .get_mut(chain_id)
            .expect("Should have the space gateway we receive an event from.");
        match request {
            // Handle Space's DHT request
            // ==========================
            GatewayRequestToParent::Dht(dht_request) => {
                match dht_request {
                    DhtRequestToParent::GossipTo(gossip_data) => {
                        handle_gossipTo(space_gateway, gossip_data)
                            .expect("Failed to gossip with space_gateway");
                    }
                    DhtRequestToParent::GossipUnreliablyTo(_data) => {
                        // n/a - should have been handled by gateway
                    }
                    // HoldPeerRequested from gossip
                    DhtRequestToParent::HoldPeerRequested(peer_data) => {
                        debug!(
                            "{} -- ({}).post() HoldPeer {:?}",
                            self.name,
                            space_gateway.as_ref().identifier(),
                            peer_data,
                        );
                        // For now accept all request
                        space_gateway.publish(GatewayRequestToChild::Dht(
                            DhtRequestToChild::HoldPeer(peer_data),
                        ));
                    }
                    DhtRequestToParent::PeerTimedOut(_peer_address) => {
                        // no-op
                    }
                    // HoldEntryRequested from gossip
                    // -> Send each aspect to Core for validation
                    DhtRequestToParent::HoldEntryRequested { from_peer, entry } => {
                        for aspect in entry.aspect_list {
                            let lib3h_msg = StoreEntryAspectData {
                                request_id: self.request_track.reserve(),
                                space_address: chain_id.0.clone(),
                                provider_agent_id: from_peer.clone().into(),
                                entry_address: entry.entry_address.clone(),
                                entry_aspect: aspect,
                            };
                            // TODO - not sure what core is expected to send back here
                            //      - right now these tracks will timeout
                            self.request_track.set(
                                &lib3h_msg.request_id,
                                Some(RealEngineTrackerData::HoldEntryRequested),
                            );
                            outbox.push(Lib3hServerProtocol::HandleStoreEntryAspect(lib3h_msg))
                        }
                    }
                    DhtRequestToParent::EntryPruned(_address) => {
                        // TODO #174
                    }
                    // EntryDataRequested: Change it into a Lib3hServerProtocol::HandleFetchEntry.
                    DhtRequestToParent::RequestEntry(entry_address) => {
                        let msg_data = FetchEntryData {
                            space_address: chain_id.0.clone(),
                            entry_address: entry_address.clone(),
                            request_id: "FIXME".to_string(),
                            provider_agent_id: chain_id.1.clone(),
                            aspect_address_list: None,
                        };
                        outbox.push(Lib3hServerProtocol::HandleFetchEntry(msg_data))
                    }
                }
            }
            // Handle Space's Transport request
            // ================================
            GatewayRequestToParent::Transport(_transport_request) => {
                // FIXME
            }
            // Handle Space's Other request
            // ============================
            _ => (), // FIXME
        }
        Ok(outbox)
    }
}
