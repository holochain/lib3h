#![allow(non_snake_case)]

use crate::{
    dht::{
        dht_protocol::{self, *},
        dht_trait::Dht,
    },
    engine::{p2p_protocol::P2pProtocol, real_engine::RealEngine, ChainId, RealEngineConfig},
    gateway::p2p_gateway::P2pGateway,
    transport::{protocol::*, transport_trait::Transport},
};
use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol, Address, AddressRef, DidWork, Lib3hResult,
};
use rmp_serde::{Deserializer, Serializer};

/// Private
impl<'t, T: Transport, D: Dht> RealEngine<'t, T, D> {
    /// Process all space gateways
    pub(crate) fn process_space_gateways(&mut self) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        // Process all gateways' DHT
        let mut outbox = Vec::new();
        for (chain_id, space_gateway) in self.space_gateway_map.iter_mut() {
            let (did_work, mut event_list) = Dht::process(space_gateway)?;
            if did_work {
                for evt in event_list {
                    let mut output = self.handle_spaceDhtEvent(chain_id, evt)?;
                    outbox.append(&mut output);
                }
            }
        }
        Ok(outbox)
    }

    /// Handle a DhtEvent sent to us by our internal DHT.
    fn handle_spaceDhtEvent(
        &mut self,
        chain_id: &ChainId,
        cmd: DhtEvent,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        let mut outbox = Vec::new();
        match cmd {
            DhtEvent::GossipTo(data) => {
                // FIXME
            }
            DhtEvent::GossipUnreliablyTo(_data) => {
                // FIXME
            }
            DhtEvent::HoldPeerRequested(_peer_address) => {
                // FIXME
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
                            space_address: chain_id.0,
                            provider_agent_id: from.as_bytes().to_vec(),
                            entry_address: entry.entry_address,
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
