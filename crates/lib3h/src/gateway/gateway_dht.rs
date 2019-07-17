#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::{p2p_protocol::*, NETWORK_GATEWAY_ID},
    gateway::{self, P2pGateway},
    transport::transport_trait::Transport,
};
use lib3h_protocol::{Address, DidWork, Lib3hResult};
use rmp_serde::Serializer;
use serde::Serialize;

/// Compose DHT
impl<T: Transport, D: Dht> Dht for P2pGateway<T, D> {
    /// Peer info
    fn get_peer_list(&self) -> Vec<PeerData> {
        self.inner_dht.get_peer_list()
    }
    fn get_peer(&self, peer_address: &str) -> Option<PeerData> {
        self.inner_dht.get_peer(peer_address)
    }
    fn this_peer(&self) -> &PeerData {
        self.inner_dht.this_peer()
    }
    /// Entry
    fn get_entry_address_list(&self) -> Vec<&Address> {
        self.inner_dht.get_entry_address_list()
    }
    fn get_aspects_of(&self, entry_address: &Address) -> Option<Vec<Address>> {
        self.inner_dht.get_aspects_of(entry_address)
    }

    /// Processing
    fn post(&mut self, cmd: DhtCommand) -> Lib3hResult<()> {
        // Add to connection_map for space_gateways
        // TODO #176 - Maybe we shouldn't have different code paths for populating
        // the connection_map between space and network gateways.
        if self.identifier != NETWORK_GATEWAY_ID {
            if let DhtCommand::HoldPeer(peer_data) = cmd.clone() {
                debug!(
                    "({}).Dht.post(HoldPeer) - {}",
                    self.identifier.clone(),
                    peer_data.peer_uri.clone()
                );
                let maybe_previous = self.connection_map.insert(
                    peer_data.peer_uri.clone(),
                    gateway::url_to_transport_id(&peer_data.peer_uri.clone()),
                );
                if let Some(previous_cId) = maybe_previous {
                    debug!(
                        "Replaced connectionId for {} ; was: {}",
                        peer_data.peer_uri.clone(),
                        previous_cId
                    );
                }
            }
        }
        self.inner_dht.post(cmd)
    }
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)> {
        // Process the dht
        let (did_work, dht_event_list) = self.inner_dht.process()?;
        trace!(
            "({}).Dht.process() - output: {} {}",
            self.identifier.clone(),
            did_work,
            dht_event_list.len()
        );
        // Handle events directly
        if did_work {
            for evt in dht_event_list.clone() {
                self.handle_DhtEvent(evt)?;
            }
        }
        // TODO #173: Check for timeouts of own requests here?
        // Done
        Ok((did_work, dht_event_list))
    }
}

/// Private internals
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    /// Handle a DhtEvent sent to us by our internal DHT.
    pub(crate) fn handle_DhtEvent(&mut self, evt: DhtEvent) -> Lib3hResult<()> {
        trace!("({}).handle_DhtEvent() {:?}", self.identifier.clone(), evt);
        match evt {
            DhtEvent::GossipTo(data) => {
                // DHT should give us the peer_transport
                for to_peer_address in data.peer_address_list {
                    // TODO #150 - should not gossip to self in the first place
                    let me = &self.inner_dht.this_peer().peer_address;
                    if &to_peer_address == me {
                        continue;
                    }
                    // TODO END
                    // Convert DHT Gossip to P2P Gossip
                    let p2p_gossip = P2pProtocol::Gossip(GossipData {
                        space_address: self.identifier().into(),
                        to_peer_address: to_peer_address.clone().into(),
                        from_peer_address: self.this_peer().peer_address.clone().into(),
                        bundle: data.bundle.clone(),
                    });
                    let mut payload = Vec::new();
                    p2p_gossip
                        .serialize(&mut Serializer::new(&mut payload))
                        .unwrap();
                    let to_conn_id = self
                        .get_connection_id(&to_peer_address)
                        .expect("Should gossip to a known peer");
                    // Forward gossip to the inner_transport
                    self.inner_transport
                        .borrow_mut()
                        .send(&[&to_conn_id], &payload)?;
                }
            }
            DhtEvent::GossipUnreliablyTo(_data) => {
                // TODO #171
            }
            DhtEvent::HoldPeerRequested(_peer_data) => {
                // no-op
            }
            DhtEvent::PeerTimedOut(_peer_address) => {
                // no-op
            }
            DhtEvent::HoldEntryRequested(_from, _data) => {
                // no-op
            }
            DhtEvent::FetchEntryResponse(_data) => {
                // no-op
            }
            DhtEvent::EntryPruned(_address) => {
                // no-op
            }
            DhtEvent::EntryDataRequested(_) => {
                // no-op
            }
        }
        Ok(())
    }
}
