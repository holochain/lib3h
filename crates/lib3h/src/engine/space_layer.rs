#![allow(non_snake_case)]

use super::RealEngineTrackerData;
use crate::{
    dht::dht_protocol::*,
    engine::{ghost_engine::handle_gossip_to, p2p_protocol::SpaceAddress, ChainId, GhostEngine},
    gateway::{protocol::*, P2pGateway},
};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{
    data_types::*, error::Lib3hProtocolResult, protocol_server::Lib3hServerProtocol,
};
use lib3h_tracing::Lib3hSpan;
use std::collections::HashMap;

/// Space layer related private methods
/// Engine does not process a space gateway's Transport because it is shared with the network layer
impl<'engine> GhostEngine<'engine> {
    /// Return list of space+this_peer for all currently joined Spaces
    pub fn get_all_spaces(&mut self) -> Vec<(SpaceAddress, PeerData)> {
        let mut result = Vec::new();
        let chain_id_list: Vec<ChainId> = self
            .space_gateway_map
            .iter()
            .map(|(id, _)| id.clone())
            .collect();
        for chainId in chain_id_list {
            let space_address: String = chainId.0.clone().into();
            result.push((space_address, self.this_space_peer(chainId.clone()).clone()));
        }
        result
    }

    /// Return first space gateway for a specified space_address
    pub fn get_first_space_mut(
        &mut self,
        space_address: &str,
    ) -> Option<&mut GatewayParentWrapper<GhostEngine<'engine>, P2pGateway>> {
        for (chainId, space_gateway) in self.space_gateway_map.iter_mut() {
            let current_space_address: String = chainId.0.clone().into();
            if current_space_address == space_address {
                return Some(space_gateway);
            }
        }
        None
    }

    /// Process all space gateways
    #[allow(dead_code)] //FIXME #324
    pub(crate) fn process_space_gateways(
        &mut self,
    ) -> Lib3hProtocolResult<Vec<Lib3hServerProtocol>> {
        // Process all space gateways and collect requests
        let mut outbox = Vec::new();
        let mut space_outbox_map = HashMap::new();
        let mut space_gateway_map: HashMap<
            ChainId,
            Detach<GatewayParentWrapper<GhostEngine<'engine>, P2pGateway>>,
        > = self.space_gateway_map.drain().collect();
        for (chain_id, mut space_gateway) in space_gateway_map.drain() {
            detach_run!(space_gateway, |g| g.process(self)).unwrap(); // FIXME unwrap
            let request_list = space_gateway.drain_messages();
            space_outbox_map.insert(chain_id.clone(), request_list);
            self.space_gateway_map.insert(chain_id, space_gateway);
            //            // FIXME: DHT magic
            //            let mut temp = space_gateway.drain_dht_outbox();
            //            self.temp_outbox.append(&mut temp);
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
                        handle_gossip_to(
                            &chain_id.0.to_string(),
                            space_gateway.as_mut(),
                            gossip_data,
                        )
                        .expect("Failed to gossip with space_gateway");
                    }
                    DhtRequestToParent::GossipUnreliablyTo(_data) => {
                        // n/a - should have been handled by gateway
                    }
                    // HoldPeerRequested from gossip
                    DhtRequestToParent::HoldPeerRequested(peer_data) => {
                        debug!(
                            "{} -- ({}).post() HoldPeer {:?}",
                            self.name, chain_id.0, peer_data,
                        );
                        // For now accept all request
                        let _res = space_gateway.publish(
                            Lib3hSpan::todo(),
                            GatewayRequestToChild::Dht(DhtRequestToChild::HoldPeer(peer_data)),
                        );
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
        }
        Ok(outbox)
    }
}
