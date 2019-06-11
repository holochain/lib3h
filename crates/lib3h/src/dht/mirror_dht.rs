use crate::dht::{
    dht_event::{DhtEvent, PeerHoldRequestData},
    dht_trait::Dht,
};
use lib3h_protocol::{AddressRef, DidWork, Lib3hResult,Address, data_types::EntryData};
use std::collections::VecDeque;

/// Mirror DHT implementation: Reflect everything back
pub struct MirrorDht {
    /// FIFO of DhtEvents send to us
    inbox: VecDeque<DhtEvent>,
    /// Storage of PeerHoldRequestData
    peer_list: HashMap<String, PeerHoldRequestData>,
    /// Storage of EntryData with empty aspect content?
    data_list: HashMap<Address, EntryData>,
    /// PeerHoldRequestData of this peer
    this_peer: PeerHoldRequestData,

}

impl MirrorDht {
    pub fn new(peer_address: &str, peer_transport: &str) -> Self {
        MirrorDht {
            inbox: VecDeque::new(),
            peer_list: HashMap::new(),
            data_list: HashMap::new(),
            this_peer: PeerHoldRequestData {
                peer_address: peer_address.to_string(),
                transport: peer_transport.to_string(),
                timestamp: 0.0,
            },
        }
    }
}

impl Dht for MirrorDht {
    // -- Getters -- //

    fn this_peer(&self) -> Lib3hResult<&str> {
        Ok((self.this_peer.peer_address))
    }

    // -- Peer -- //

    fn get_peer(&self, peer_address: &str) -> Option<PeerHoldRequestData> {
        self.peer_list.get(peer_address)
    }
    fn fetch_peer(&self, peer_address: &str) -> Option<PeerHoldRequestData> {
        // FIXME: wait 200ms
        self.get_peer(data_address)
    }
    fn drop_peer(&self, peer_address: &str) -> Lib3hResult<()> {
        self.peer_list.remove(peer_address)
    }
    fn get_peer_list(&self) -> Vec<PeerHoldRequestData> {
        self.peer_list.iter().collect()
    }

    // -- Data -- //

    fn get_data(&self, data_address: &AddressRef) -> Lib3hResult<Vec<u8>> {
        let entry = self.data_list
            .get(data_address)
            .ok_or(Err(format_err!("No data at data_address")))?;
        let mut buf = Vec::new();
        entry.serialize(&mut Serializer::new(&mut buf)).unwrap();
        Ok(buf)
    }

    /// Same as get_data with artificial wait
    fn fetch_data(&self, data_address: &AddressRef) -> Lib3hResult<Vec<u8>> {
        // FIXME: wait 200ms
        self.get_data(data_address)
    }

    // -- Processing -- //

    fn post(&mut self, evt: DhtEvent) -> Lib3hResult<()> {
        self.inbox.push_back(evt);
        Ok(())
    }

    ///
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)> {
        let outbox = Vec::new();
        let mut did_work = false;
        loop {
            let evt = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            let res = self.serve_DhtEvent(&evt);
            if let Ok(mut output) = res {
                did_work = true;
                outbox.append(&mut output);
            }
        }
        Ok((did_work, outbox))
    }
}


impl MirrorDht {

    fn add_data() {

    }

    /// Process a DhtEvent Command, sent by our owner.
    /// Return a list of DhtEvent to owner.
    #[allow(non_snake_case)]
    fn serve_DhtEvent(
        &mut self,
        evt: &DhtEvent,
    ) -> TransportResult<Vec<DhtEvent>> {
        println!("(log.d) >>> '(MirrorDht)' recv evt: {:?}", evt);
        // Note: use same order as the enum
        match evt {
            /// Received gossip from remote node:
            /// - DataHoldRequestData
            /// - PeerHoldRequestData
            DhtEvent::RemoteGossipBundle(msg) => {
                let mut de = Deserializer::new(&buf[..]);
                let data: DataHoldRequestData = Deserialize::deserialize(&mut de)();
                let evt = DhtEvent::DataHoldRequest(data);
                let data: PeerHoldRequestData = Deserialize::deserialize(&mut de)();
                let evt = DhtEvent::PeerHoldRequest(data);
                let res = self.serve_DhtEvent(evt);
                res
            }
            // N/A
            DhtEvent::GossipTo(_) => {
                panic!("Should not receive this?");
                Ok(vec![])
            }
            // N/A
            DhtEvent::UnreliableGossipTo(_) => {
                panic!("Should not receive this?");
                Ok(vec![])
            }
            /// Owner is asking us to hold peer info
            DhtEvent::PeerHoldRequest(msg) => {
                // Store it
                let mut maybe_peer = self.peer_list.get_mut(msg.peer_address);
                match maybe_peer {
                    None => {
                        self.peer_list.insert(msg.peer_address, msg);
                    }
                    Some(mut peer) => {
                        if msg.timestamp > peer.timestamp {
                            peer.timestamp = msg.timestamp;
                        }
                    }
                };
                let peer = self.peer_list.get(msg.peer_address).expect("Should have peer by now");
                let mut buf = Vec::new();
                peer.serialize(&mut Serializer::new(&mut buf)).unwrap();
                // Gossip to everyone to also hold it
                let gossip_evt = GossipToData {
                    peer_address_list: self.get_peer_list(),
                    bundle: buf,
                };
                gossip_list.push(DhtEvent::GossipTo(gossip_evt));
                // Done
                Ok(vec![gossip_evt])
            }
            // N/A
            DhtEvent::PeerTimedOut(_) => {Ok(vec![])}
            /// Owner asks us to hold some data. Store it and gossip to every known peer.
            DhtEvent::DataHoldRequest(msg) => {
                // Local asks us to hold data.
                // Store it
                let mut received_new_content = false;
                let maybe_entry = self.data_list.get_mut(msg.entry.entry_address);
                match maybe_entry {
                    None => {
                        self.data_list.insert(msg.entry.entry_address, msg.entry);
                        received_new_content = true;
                    }
                    Some(mut entry) => {
                        received_new_content = entry.merge(msg.entry);
                    }
                };
                // Bail if did not receive new content
                if !received_new_content {
                    Ok(vec![])
                }
                // Gossip data to all known peers
                let entry = self.data_list.get(msg.entry.entry_address).expect("Should have content at this point");
                let mut buf = Vec::new();
                entry.serialize(&mut Serializer::new(&mut buf)).unwrap();
                let gossip_evt = GossipToData {
                    peer_address_list: self.get_peer_list(),
                    bundle: buf,
                };
                gossip_list.push(DhtEvent::GossipTo(gossip_evt));
                // Done
                Ok(vec![gossip_evt])
            }
            DhtEvent::DataFetch(_) => {
                panic!("Should not receive this?");
                Ok(vec![])
            }
            DhtEvent::DataFetchResponse(_) => {
                panic!("Should not receive this?");
                Ok(vec![])
            }
            // Monotonic fullsync dht for now
            DhtEvent::DataPrune(_) => {
                panic!("Should not receive this?");
                Ok(vec![])
            }
        }
    }
}