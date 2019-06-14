#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht, rrdht::RrDht},
    engine::p2p_protocol::*,
    gateway::p2p_gateway::P2pGateway,
    transport::transport_trait::Transport,
};
use lib3h_protocol::{data_types::EntryData, Address, AddressRef, DidWork, Lib3hResult};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

/// Compose DHT
impl<'t, T: Transport, D: Dht> Dht for P2pGateway<'t, T, D> {
    //impl<'t, T: Transport, D: Dht> Dht for P2pGateway<'t, P2pGateway<'t, T, D>, D> {
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
        // Handle events directly
        if did_work {
            for evt in dht_event_list {
                self.handle_DhtEvent(evt)?;
            }
        }
        Ok((did_work, dht_event_list))
    }
    /// Getters
    fn this_peer(&self) -> Lib3hResult<&str> {
        self.inner_dht.this_peer()
    }
    fn get_peer_list(&self) -> Vec<PeerData> {
        self.inner_dht.get_peer_list()
    }
}

/// Private internals
impl<'t, T: Transport, D: Dht> P2pGateway<'t, T, D> {
    /// For space gateway, space_address is stored in the advertise field.
    /// This function does this conversion.
    fn space_address(&self) -> Address {
        let advertise = self
            .maybe_advertise
            .expect("Advertise for space gateway should be set at construction");
        return advertise.as_bytes().to_vec();
    }

    /// Handle a DhtEvent sent to us by our internal DHT.
    pub(crate) fn handle_DhtEvent(&mut self, cmd: DhtEvent) -> Lib3hResult<()> {
        match cmd {
            DhtEvent::GossipTo(data) => {
                // DHT should give us the peer_transport
                for to_peer_address in data.peer_address_list {
                    // get peer address
                    let peer_transport = self
                        .inner_dht
                        .get_peer(&to_peer_address)
                        .expect("Should gossip to a known peer")
                        .transport
                        .as_str();
                    // Change into P2pProtocol
                    let p2p_gossip = P2pProtocol::Gossip(GossipData {
                        space_address: self.space_address(),
                        to_peer_address: to_peer_address.as_bytes().to_vec(),
                        from_peer_address: self.id().as_bytes().to_vec(),
                        bundle: data.bundle.clone(),
                    });
                    let mut payload = Vec::new();
                    p2p_gossip
                        .serialize(&mut Serializer::new(&mut payload))
                        .unwrap();
                    // Forward gossip to the inner_transport
                    // If no connection to that transportId is open, open one first.
                    self.inner_transport.send(&[peer_transport], &payload)?;
                }
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
            DhtEvent::HoldEntryRequested(_from, _data) => {
                // FIXME: N/A? Have engine handle it?
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
