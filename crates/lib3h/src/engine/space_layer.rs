#![allow(non_snake_case)]

use super::RealEngineTrackerData;
use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::{p2p_protocol::SpaceAddress, ChainId, RealEngine},
    gateway::GatewayWrapper,
};
use lib3h_protocol::{
    data_types::*, error::Lib3hProtocolResult, protocol_server::Lib3hServerProtocol,
};
use std::collections::HashMap;

/// Space layer related private methods
/// Engine does not process a space gateway's Transport because it is shared with the network layer
impl<'engine, D: Dht> RealEngine<'engine, D> {
    /// Return list of space+this_peer for all currently joined Spaces
    pub fn get_all_spaces(&self) -> Vec<(SpaceAddress, PeerData)> {
        let mut result = Vec::new();
        for (chainId, space_gateway) in self.space_gateway_map.iter() {
            let space_address: String = chainId.0.clone().into();
            result.push((space_address, space_gateway.as_ref().this_peer().clone()));
        }
        result
    }

    /// Return first space gateway for a specified space_address
    pub fn get_first_space_mut(&mut self, space_address: &str) -> Option<GatewayWrapper<'engine>> {
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
        // Process all gateways' DHT
        let mut outbox = Vec::new();
        let mut dht_outbox = HashMap::new();
        for (chain_id, space_gateway) in self.space_gateway_map.iter_mut() {
            let (did_work, event_list) = space_gateway.as_dht_mut().process()?;
            if did_work {
                // TODO: perf optim, don't copy chain_id
                dht_outbox.insert(chain_id.clone(), event_list);
            }
        }
        // Process all gateway DHT events
        for (chain_id, evt_list) in dht_outbox {
            for evt in evt_list {
                let mut output = self.handle_spaceDhtEvent(&chain_id, evt.clone())?;
                outbox.append(&mut output);
            }
        }
        Ok(outbox)
    }

    /// Handle a DhtEvent sent to us by a space gateway
    fn handle_spaceDhtEvent(
        &mut self,
        chain_id: &ChainId,
        cmd: DhtEvent,
    ) -> Lib3hProtocolResult<Vec<Lib3hServerProtocol>> {
        debug!(
            "{} << handle_spaceDhtEvent: [{:?}] - {:?}",
            self.name, chain_id, cmd,
        );
        let mut outbox = Vec::new();
        let space_gateway = self
            .space_gateway_map
            .get_mut(chain_id)
            .expect("Should have the space gateway we receive an event from.");
        match cmd {
            DhtEvent::GossipTo(_gossip_data) => {
                // n/a - should have been handled by gateway
            }
            DhtEvent::GossipUnreliablyTo(_data) => {
                // n/a - should have been handled by gateway
            }
            // HoldPeerRequested from gossip
            DhtEvent::HoldPeerRequested(peer_data) => {
                debug!(
                    "{} -- ({}).post() HoldPeer {:?}",
                    self.name,
                    space_gateway.as_ref().identifier(),
                    peer_data,
                );
                // For now accept all request
                space_gateway
                    .as_dht_mut()
                    .post(DhtCommand::HoldPeer(peer_data))?;
            }
            DhtEvent::PeerTimedOut(_peer_address) => {
                // no-op
            }
            // HoldEntryRequested from gossip
            // -> Send each aspect to Core for validation
            DhtEvent::HoldEntryRequested(from, entry) => {
                for aspect in entry.aspect_list {
                    let lib3h_msg = StoreEntryAspectData {
                        request_id: self.request_track.reserve(),
                        space_address: chain_id.0.clone(),
                        provider_agent_id: from.clone().into(),
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
            // FetchEntryResponse: Send back as a query response to Core
            // TODO #169 - Discern Fetch from Query
            DhtEvent::FetchEntryResponse(response) => {
                let query_result = serde_json::to_string_pretty(&response.entry)
                    .unwrap()
                    .as_bytes()
                    .to_vec();
                let msg_data = QueryEntryResultData {
                    space_address: chain_id.0.clone(),
                    entry_address: response.entry.entry_address.clone(),
                    request_id: response.msg_id.clone(),
                    requester_agent_id: chain_id.1.clone(), // TODO #150 - get requester from channel from p2p-protocol
                    responder_agent_id: chain_id.1.clone(),
                    query_result: query_result.into(),
                };
                outbox.push(Lib3hServerProtocol::QueryEntryResult(msg_data))
            }
            DhtEvent::EntryPruned(_address) => {
                // TODO #174
            }
            // EntryDataRequested: Change it into a Lib3hServerProtocol::HandleFetchEntry.
            DhtEvent::EntryDataRequested(fetch_entry) => {
                let msg_data = FetchEntryData {
                    space_address: chain_id.0.clone(),
                    entry_address: fetch_entry.entry_address.clone(),
                    request_id: fetch_entry.msg_id.clone(),
                    provider_agent_id: chain_id.1.clone(),
                    aspect_address_list: None,
                };
                outbox.push(Lib3hServerProtocol::HandleFetchEntry(msg_data))
            }
        }
        Ok(outbox)
    }
}
