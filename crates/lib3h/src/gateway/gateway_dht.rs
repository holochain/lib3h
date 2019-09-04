#![allow(non_snake_case)]

use crate::{
    dht::{
        dht_protocol::*, ghost_protocol::*,
    },
    engine::{p2p_protocol::*, NETWORK_GATEWAY_ID},
    error::Lib3hResult,
    gateway::{Gateway, P2pGateway},
};
use lib3h_protocol::{Address, DidWork};
use rmp_serde::Serializer;
use serde::Serialize;

/// Compose DHT
impl<'gateway>  P2pGateway<'gateway> {
    /// Processing
    fn post(&mut self, cmd: DhtRequestToChild) -> Lib3hResult<()> {
        // Add to connection_map for space_gateways
        // TODO #176 - Maybe we shouldn't have different code paths for populating
        // the connection_map between space and network gateways.
        if self.identifier != NETWORK_GATEWAY_ID {
            if let DhtRequestToChild::HoldPeer(peer_data) = &cmd {
                debug!(
                    "({}).Dht.post(HoldPeer) - {}",
                    self.identifier, peer_data.peer_uri,
                );
                // In space_gateway `peer_uri` is a URI-ed transportId, so un-URI-ze it
                // to get the transportId
                let maybe_previous = self.connection_map.insert(
                    peer_data.peer_uri.clone(),
                    String::from(peer_data.peer_uri.path()),
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
        self.inner_dht.publish(cmd);
        Ok(())
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
impl<'gateway> P2pGateway<'gateway> {
    /// Handle a DhtEvent sent to us by our internal DHT.
    pub(crate) fn handle_DhtEvent(&mut self, evt: DhtRequestToParent) -> Lib3hResult<()> {
        trace!("({}).handle_DhtEvent() {:?}", self.identifier, evt);
        match evt {
            DhtRequestToParent::GossipTo(data) => {
                // DHT should give us the peer_transport
                for to_peer_address in data.peer_address_list {
                    // TODO #150 - should not gossip to self in the first place
                    let me = &self.get_this_peer_sync().peer_address;
                    if &to_peer_address == me {
                        continue;
                    }
                    // TODO END
                    // Convert DHT Gossip to P2P Gossip
                    let p2p_gossip = P2pProtocol::Gossip(GossipData {
                        space_address: self.identifier().into(),
                        to_peer_address: to_peer_address.clone().into(),
                        from_peer_address: me.clone().into(),
                        bundle: data.bundle.clone(),
                    });
                    let mut payload = Vec::new();
                    p2p_gossip
                        .serialize(&mut Serializer::new(&mut payload))
                        .expect("P2pProtocol::Gossip serialization failed");
                    let to_conn_id = self
                        .get_connection_id(&to_peer_address)
                        .expect("Should gossip to a known peer");
                    // Forward gossip to the inner_transport
                    self.inner_transport
                        .as_mut()
                        .send(&[&to_conn_id], &payload)?;
                }
            }
            DhtRequestToParent::GossipUnreliablyTo(_data) => {
                // TODO #171
            }
            DhtRequestToParent::HoldPeerRequested(_peer_data) => {
                // no-op
            }
            DhtRequestToParent::PeerTimedOut(_peer_address) => {
                // no-op
            }
            DhtRequestToParent::HoldEntryRequested{from_peer, entry} => {
                // no-op
            }
            DhtRequestToParent::EntryPruned(_address) => {
                // no-op
            }
            DhtRequestToParent::RequestEntry(_) => {
                // no-op
            }
        }
        Ok(())
    }
}
