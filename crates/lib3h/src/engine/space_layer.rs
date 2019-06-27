#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::{p2p_protocol::SpaceAddress, ChainId, RealEngine},
    gateway::P2pGateway,
    transport::transport_trait::Transport,
};
use lib3h_crypto_api::{Buffer, CryptoSystem};
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol, Lib3hResult};
use std::collections::HashMap;

/// Space layer related private methods
/// Engine does not process a space gateway's Transport because it is shared with the network layer
impl<T: Transport, D: Dht, SecBuf: Buffer, Crypto: CryptoSystem> RealEngine<T, D, SecBuf, Crypto> {
    /// Return list of space+this_peer for all currently joined Spaces
    pub fn get_all_spaces(&self) -> Vec<(SpaceAddress, PeerData)> {
        let mut result = Vec::new();
        for (chainId, space_gateway) in self.space_gateway_map.iter() {
            let space_address = std::string::String::from_utf8_lossy(&chainId.0).into_owned();
            result.push((space_address, space_gateway.this_peer().clone()));
        }
        result
    }

    /// Return first space gateway for a specified space_address
    pub fn get_first_space_mut(
        &mut self,
        space_address: &str,
    ) -> Option<&mut P2pGateway<P2pGateway<T, D>, D>> {
        for (chainId, space_gateway) in self.space_gateway_map.iter_mut() {
            let current_space_address =
                std::string::String::from_utf8_lossy(&chainId.0).into_owned();
            if current_space_address == space_address {
                return Some(space_gateway);
            }
        }
        None
    }

    /// Process all space gateways
    pub(crate) fn process_space_gateways(&mut self) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        // Process all gateways' DHT
        let mut outbox = Vec::new();
        let mut dht_outbox = HashMap::new();
        for (chain_id, space_gateway) in self.space_gateway_map.iter_mut() {
            let (did_work, event_list) = Dht::process(space_gateway)?;
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
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        debug!(
            "{} << handle_spaceDhtEvent: [{:?}] - {:?}",
            self.name.clone(),
            chain_id,
            cmd
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
            DhtEvent::HoldPeerRequested(peer_data) => {
                debug!(
                    "{} -- ({}).post() HoldPeer {:?}",
                    self.name.clone(),
                    space_gateway.identifier(),
                    peer_data
                );
                // For now accept all request
                let hold_cmd = DhtCommand::HoldPeer(peer_data);
                space_gateway.post_dht(hold_cmd)?;
            }
            DhtEvent::PeerTimedOut(_data) => {
                // FIXME
            }
            DhtEvent::HoldEntryRequested(from, entry) => {
                // Send each aspect to Core for validation
                for aspect in entry.aspect_list {
                    let lib3h_msg =
                        Lib3hServerProtocol::HandleStoreEntryAspect(StoreEntryAspectData {
                            request_id: "FIXME".to_string(),
                            space_address: chain_id.0.clone(),
                            provider_agent_id: from.as_bytes().to_vec(),
                            entry_address: entry.entry_address.clone(),
                            entry_aspect: aspect,
                        });
                    outbox.push(lib3h_msg)
                }
            }
            DhtEvent::FetchEntryResponse(_data) => {
                // FIXME
            }
            DhtEvent::EntryPruned(_address) => {
                // FIXME
            }
        }
        Ok(outbox)
    }
}
