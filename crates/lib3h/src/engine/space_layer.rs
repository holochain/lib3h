#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::{ChainId, RealEngine},
    transport::transport_trait::Transport,
};
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol, Lib3hResult};
use std::collections::HashMap;

/// Private
impl<T: Transport, D: Dht> RealEngine<T, D> {
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

    /// Handle a DhtEvent sent to us by our internal DHT.
    fn handle_spaceDhtEvent(
        &mut self,
        chain_id: &ChainId,
        cmd: DhtEvent,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        println!(
            "[d] {} << handle_spaceDhtEvent: [{:?}] - {:?}",
            self.name.clone(),
            chain_id,
            cmd
        );
        let mut outbox = Vec::new();
        match cmd {
            DhtEvent::GossipTo(_data) => {
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
