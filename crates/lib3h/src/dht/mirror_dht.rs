use crate::dht::{
    dht_protocol::*,
    dht_trait::{Dht, DhtConfig},
};
use lib3h_protocol::{data_types::EntryData, Address, AddressRef, DidWork, Lib3hResult};
use std::collections::{HashMap, VecDeque};

use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;

/// Enum holding all types of gossip messages used by MirrorDht
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
enum MirrorGossip {
    Entry(EntryData),
    Peer(PeerData),
}

/// Mirror DHT implementation: Holds and reflect everything back to other nodes (fullsync)
///  - On *HoldRequest, store and gossip data back to every known peer.
///  - Gossip can only be a *HoldRequest
///  - Monotonic data
pub struct MirrorDht {
    /// FIFO of DhtCommands send to us
    inbox: VecDeque<DhtCommand>,
    /// Storage of PeerData
    peer_list: HashMap<String, PeerData>,
    /// Storage of EntryData with empty aspect content?
    entry_list: HashMap<Address, EntryData>,
    /// PeerData of this peer
    this_peer: PeerData,
}

/// Constructors
impl MirrorDht {
    pub fn new(peer_address: &str, peer_transport: &Url) -> Self {
        MirrorDht {
            inbox: VecDeque::new(),
            peer_list: HashMap::new(),
            entry_list: HashMap::new(),
            this_peer: PeerData {
                peer_address: peer_address.to_string(),
                transport: peer_transport.clone(),
                timestamp: 0, // FIXME
            },
        }
    }

    pub fn new_with_config(config: &DhtConfig) -> Lib3hResult<Self> {
        Ok(Self::new(
            &config.this_peer_address,
            &config.this_peer_transport,
        ))
    }
}

/// Impl Dht interface
impl Dht for MirrorDht {
    // -- Getters -- //

    fn this_peer(&self) -> &PeerData {
        &self.this_peer
    }

    // -- Peer -- //

    fn get_peer(&self, peer_address: &str) -> Option<PeerData> {
        let res = self.peer_list.get(peer_address);
        if let Some(pd) = res {
            return Some(pd.clone());
        }
        None
    }
    fn fetch_peer(&self, peer_address: &str) -> Option<PeerData> {
        // FIXME: wait 200ms?
        self.get_peer(peer_address)
    }

    fn get_peer_list(&self) -> Vec<PeerData> {
        self.peer_list.values().map(|v| v.clone()).collect()
    }

    // -- Data -- //

    fn get_entry(&self, data_address: &AddressRef) -> Option<EntryData> {
        self.entry_list.get(data_address).map(|e| e.clone())
    }

    /// Same as get_data with artificial wait
    fn fetch_entry(&self, data_address: &AddressRef) -> Option<EntryData> {
        // FIXME: wait 200ms?
        self.get_entry(data_address)
    }

    // -- Processing -- //

    fn post(&mut self, cmd: DhtCommand) -> Lib3hResult<()> {
        // println!("(log.d) >>> '(MirrorDht)' recv cmd: {:?}", cmd);
        self.inbox.push_back(cmd);
        Ok(())
    }

    ///
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)> {
        let mut outbox = Vec::new();
        let mut did_work = false;
        loop {
            let cmd = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            let res = self.serve_DhtCommand(&cmd);
            if let Ok(mut output) = res {
                did_work = true;
                outbox.append(&mut output);
            } else {
                println!("[e] serve_DhtCommand() failed: {:?}", res);
            }
        }
        Ok((did_work, outbox))
    }
}

/// Internals
impl MirrorDht {
    /// Return true if new peer or updated peer
    fn add_peer(&mut self, peer_info: &PeerData) -> bool {
        println!("[t] @MirrorDht@ Adding peer: {:?}", peer_info);
        let maybe_peer = self.peer_list.get_mut(&peer_info.peer_address);
        match maybe_peer {
            None => {
                println!("[t] @MirrorDht@ Adding peer - OK NEW");
                self.peer_list
                    .insert(peer_info.peer_address.clone(), peer_info.clone());
                true
            }
            Some(mut peer) => {
                if peer_info.timestamp <= peer.timestamp {
                    println!("[t] @MirrorDht@ Adding peer - BAD");
                    return false;
                }
                println!("[t] @MirrorDht@ Adding peer - OK UPDATED");
                peer.timestamp = peer_info.timestamp;
                true
            }
        }
    }

    /// Add entry to our local storage
    /// Return true if new entry was successfully added
    fn add_entry(&mut self, entry: &EntryData) -> bool {
        let maybe_entry = self.entry_list.get_mut(&entry.entry_address);
        match maybe_entry {
            None => {
                self.entry_list
                    .insert(entry.entry_address.clone(), entry.clone());
                true
            }
            Some(prev_entry) => {
                return prev_entry.merge(entry);
            }
        }
    }

    /// Process a DhtEvent Command, sent by our owner.
    /// Return a list of DhtEvent to owner.
    #[allow(non_snake_case)]
    fn serve_DhtCommand(&mut self, cmd: &DhtCommand) -> Lib3hResult<Vec<DhtEvent>> {
        println!("[d] @MirrorDht@ serving cmd: {:?}", cmd);
        // Note: use same order as the enum
        match cmd {
            // Received gossip from remote node. Bundle must be a serialized MirrorGossip
            DhtCommand::HandleGossip(msg) => {
                println!("Deserializer msg.bundle: {:?}", msg.bundle);
                let mut de = Deserializer::new(&msg.bundle[..]);
                let maybe_gossip: Result<MirrorGossip, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                if let Err(e) = maybe_gossip {
                    return Err(format_err!("Failed deserializing gossip: {:?}", e));
                }
                match maybe_gossip.unwrap() {
                    MirrorGossip::Entry(entry) => {
                        let is_new = self.add_entry(&entry);
                        if is_new {
                            return Ok(vec![DhtEvent::HoldEntryRequested(
                                self.this_peer.peer_address.clone(),
                                entry,
                            )]);
                        }
                        return Ok(vec![]);
                    }
                    MirrorGossip::Peer(peer) => {
                        let is_new = self.add_peer(&peer);
                        if is_new {
                            return Ok(vec![DhtEvent::HoldPeerRequested(peer)]);
                        }
                        return Ok(vec![]);
                    }
                }
            }
            // Owner is asking us to hold peer info
            DhtCommand::HoldPeer(msg) => {
                // Get peer_list before adding new peer (to use when doing gossipTo)
                let peer_address_list: Vec<String> = self
                    .get_peer_list()
                    .iter()
                    .map(|pi| pi.peer_address.clone())
                    .collect();
                // Store it
                let received_new_content = self.add_peer(msg);
                // Bail if peer is known and up to date.
                if !received_new_content {
                    return Ok(vec![]);
                }
                let mut event_list = Vec::new();
                // Gossip to everyone to also hold it
                let peer = self
                    .peer_list
                    .get(&msg.peer_address)
                    .expect("Should have peer by now");
                let peer_gossip = MirrorGossip::Peer(peer.clone());
                let mut buf = Vec::new();
                peer_gossip
                    .serialize(&mut Serializer::new(&mut buf))
                    .unwrap();
                println!("[t] @MirrorDht@ gossiping peer: {:?}", peer);
                let gossip_evt = GossipToData {
                    peer_address_list,
                    bundle: buf,
                };
                event_list.push(DhtEvent::GossipTo(gossip_evt));

                // Gossip back your own PeerData
                let peer = self.this_peer();
                if msg.peer_address != peer.peer_address {
                    let peer_gossip = MirrorGossip::Peer(peer.clone());
                    let mut buf = Vec::new();
                    peer_gossip
                        .serialize(&mut Serializer::new(&mut buf))
                        .unwrap();
                    println!(
                        "[t] @MirrorDht@ gossiping peer back: {:?} | to: {}",
                        peer, msg.peer_address
                    );
                    let gossip_evt = GossipToData {
                        peer_address_list: vec![msg.peer_address.clone()],
                        bundle: buf,
                    };
                    event_list.push(DhtEvent::GossipTo(gossip_evt));
                }
                // Done
                Ok(event_list)
            }
            // Owner asks us to hold some entry. Store it.
            DhtCommand::HoldEntry(entry) => {
                // Store it
                let _received_new_content = self.add_entry(entry);
                Ok(vec![])
            }
            // Owner asks us to hold some entry. Store it and gossip to every known peer.
            DhtCommand::BroadcastEntry(entry) => {
                // Local asks us to hold entry.
                // Store it
                let received_new_content = self.add_entry(entry);
                // Bail if did not receive new content
                if !received_new_content {
                    return Ok(vec![]);
                }
                // Gossip new entry to all known peers
                let entry = self
                    .entry_list
                    .get(&entry.entry_address)
                    .expect("Should have content at this point");
                let entry_gossip = MirrorGossip::Entry(entry.clone());
                let mut buf = Vec::new();
                entry_gossip
                    .serialize(&mut Serializer::new(&mut buf))
                    .unwrap();
                let gossip_evt = GossipToData {
                    peer_address_list: self
                        .get_peer_list()
                        .iter()
                        .map(|pi| pi.peer_address.clone())
                        .collect(),
                    bundle: buf,
                };
                // Done
                Ok(vec![DhtEvent::GossipTo(gossip_evt)])
            }
            // Respond with internal query
            DhtCommand::FetchEntry(msg) => {
                let maybe_entry = self.fetch_entry(&msg.entry_address);
                match maybe_entry {
                    None => Ok(vec![]),
                    Some(entry) => {
                        let response = FetchEntryResponseData {
                            msg_id: msg.msg_id.clone(),
                            entry,
                        };
                        Ok(vec![DhtEvent::FetchEntryResponse(response)])
                    }
                }
            }
            // Monotonic fullsync dht for now
            DhtCommand::DropEntry(_) => Ok(vec![]),
        }
    }
}
