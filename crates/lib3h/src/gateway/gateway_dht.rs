#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::p2p_protocol::*,
    gateway::P2pGateway,
    transport::transport_trait::Transport,
};
use lib3h_protocol::{data_types::EntryData, AddressRef, DidWork, Lib3hResult};
use rmp_serde::Serializer;
use serde::Serialize;

/// Compose DHT
impl<T: Transport, D: Dht> Dht for P2pGateway<T, D> {
    /// Peer info
    fn get_peer(&self, peer_address: &str) -> Option<PeerData> {
        self.inner_dht.get_peer(peer_address)
    }
    fn fetch_peer(&self, peer_address: &str) -> Option<PeerData> {
        self.inner_dht.fetch_peer(peer_address)
    }
    /// Entry
    fn get_entry(&self, entry_address: &AddressRef) -> Option<EntryData> {
        self.inner_dht.get_entry(entry_address)
    }
    fn fetch_entry(&self, entry_address: &AddressRef) -> Option<EntryData> {
        self.inner_dht.fetch_entry(entry_address)
    }
    /// Processing
    fn post(&mut self, cmd: DhtCommand) -> Lib3hResult<()> {
        self.inner_dht.post(cmd)
    }
    /// FIXME: should P2pGateway `post() & process()` its inner dht?
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)> {
        // Process the dht
        let (did_work, dht_event_list) = self.inner_dht.process()?;
        debug!(
            "[t] ({}).Dht.process() - output: {} {}",
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
        Ok((did_work, dht_event_list))
    }
    /// Getters
    fn this_peer(&self) -> &PeerData {
        self.inner_dht.this_peer()
    }
    fn get_peer_list(&self) -> Vec<PeerData> {
        self.inner_dht.get_peer_list()
    }
}

/// Private internals
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    /// Handle a DhtEvent sent to us by our internal DHT.
    pub(crate) fn handle_DhtEvent(&mut self, evt: DhtEvent) -> Lib3hResult<()> {
        debug!(
            "[t] ({}).handle_DhtEvent() {:?}",
            self.identifier.clone(),
            evt
        );
        match evt {
            DhtEvent::GossipTo(data) => {
                // DHT should give us the peer_transport
                for to_peer_address in data.peer_address_list {
                    // HACK: (should not gossip to self in the first place)
                    let me = &self.inner_dht.this_peer().peer_address;
                    if &to_peer_address == me {
                        continue;
                    }
                    // get peer address
                    let peer_transport = self
                        .inner_dht
                        .get_peer(&to_peer_address)
                        .expect("Should gossip to a known peer")
                        .peer_uri;
                    debug!(
                        "({}) GossipTo: {} {}",
                        self.identifier.clone(),
                        to_peer_address,
                        peer_transport
                    );
                    // Change into P2pProtocol
                    let p2p_gossip = P2pProtocol::Gossip(GossipData {
                        space_address: self.identifier().as_bytes().to_vec(),
                        to_peer_address: to_peer_address.as_bytes().to_vec(),
                        from_peer_address: self.this_peer().peer_address.as_bytes().to_vec(),
                        bundle: data.bundle.clone(),
                    });
                    let mut payload = Vec::new();
                    p2p_gossip
                        .serialize(&mut Serializer::new(&mut payload))
                        .unwrap();
                    // Forward gossip to the inner_transport
                    // If no connection to that connectionId is open, open one first.
                    self.inner_transport
                        .borrow_mut()
                        .send(&[peer_transport.path()], &payload)?;
                }
            }
            DhtEvent::GossipUnreliablyTo(_data) => {
                // FIXME
            }
            DhtEvent::HoldPeerRequested(_peer_address) => {
                // FIXME or have engine handle it?
            }
            DhtEvent::PeerTimedOut(_data) => {
                // FIXME
            }
            DhtEvent::HoldEntryRequested(_from, _data) => {
                // N/A - Have engine handle it
            }
            DhtEvent::FetchEntryResponse(_data) => {
                // FIXME
            }
            DhtEvent::EntryPruned(_address) => {
                // FIXME
            }
        }
        Ok(())
    }
}
