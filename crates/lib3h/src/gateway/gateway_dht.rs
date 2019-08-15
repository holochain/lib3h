#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::{p2p_protocol::*, NETWORK_GATEWAY_ID},
    error::Lib3hResult,
    gateway::{Gateway, P2pGateway},
    transport::transport_trait::Transport,
};
use lib3h_protocol::{Address, DidWork};
use rmp_serde::Serializer;
use serde::Serialize;
use url::Url;

/// Compose DHT
impl<'gateway, D: Dht> Dht for P2pGateway<'gateway, D> {
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
                    self.identifier, peer_data.peer_uri,
                );
                // In space_gateway `peer_uri` is a URI-ed transportId, so un-URI-ze it
                // to get the transportId
                self.connections.insert(peer_data.peer_uri.clone());
            }
        }
        self.inner_dht.post(cmd)
    }
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)> {
        // Process the dht
        let (did_work, dht_event_list) = self.inner_dht.process()?;
        trace!(
            "({}).Dht.process() - output: {} {}",
            self.identifier,
            did_work,
            dht_event_list.len(),
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
impl<'gateway, D: Dht> P2pGateway<'gateway, D> {
    /// Handle a DhtEvent sent to us by our internal DHT.
    pub(crate) fn handle_DhtEvent(&mut self, evt: DhtEvent) -> Lib3hResult<()> {
        trace!("({}).handle_DhtEvent() {:?}", self.identifier, evt);
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
                        .expect("P2pProtocol::Gossip serialization failed");
                    self.send(
                        "".to_string(),
                        Url::parse(&format!("{}:{}", self.address_url_scheme, to_peer_address))
                            .expect("can parse url"),
                        payload,
                    )?;
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
