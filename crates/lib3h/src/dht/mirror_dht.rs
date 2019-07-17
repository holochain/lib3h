use crate::{
    dht::{
        dht_protocol::*,
        dht_trait::{Dht, DhtConfig},
    },
    time,
};
use lib3h_protocol::{data_types::EntryData, Address, DidWork, Lib3hResult};
use std::collections::{HashMap, HashSet, VecDeque};

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
    /// Storage of EntryData with empty aspect content?
    entry_list: HashMap<Address, HashSet<Address>>,
    /// Storage of PeerData
    peer_list: HashMap<String, PeerData>,
    /// PeerData of this peer
    this_peer: PeerData,
    /// Keep track of fetch requests sent to Core
    pending_fetch_request_list: HashSet<String>,

    last_gossip_of_self: u64,

    config: DhtConfig,
}

/// Constructors
impl MirrorDht {
    pub fn new(peer_address: &str, peer_uri: &Url) -> Self {
        let dht_config = DhtConfig::new(peer_address, peer_uri);
        Self::new_with_config(&dht_config).expect("Failed creating default MirrorDht")
    }

    pub fn new_with_config(config: &DhtConfig) -> Lib3hResult<Self> {
        let timestamp = time::since_epoch_ms();
        let this = MirrorDht {
            inbox: VecDeque::new(),
            peer_list: HashMap::new(),
            entry_list: HashMap::new(),
            this_peer: PeerData {
                peer_address: config.this_peer_address.to_string(),
                peer_uri: config.this_peer_uri.clone(),
                timestamp,
            },
            pending_fetch_request_list: HashSet::new(),
            last_gossip_of_self: timestamp,
            config: config.clone(),
        };
        Ok(this)
    }
}

/// Impl Dht interface
impl Dht for MirrorDht {
    // -- Peer info -- //

    fn get_peer_list(&self) -> Vec<PeerData> {
        self.peer_list.values().map(|v| v.clone()).collect()
    }

    fn get_peer(&self, peer_address: &str) -> Option<PeerData> {
        println!("get_peer({}) in {:?}", peer_address, self.peer_list);
        let res = self.peer_list.get(peer_address);
        if let Some(pd) = res {
            return Some(pd.clone());
        }
        None
    }

    fn this_peer(&self) -> &PeerData {
        &self.this_peer
    }

    // -- Entry -- //

    fn get_entry_address_list(&self) -> Vec<&Address> {
        self.entry_list.iter().map(|kv| kv.0).collect()
    }

    fn get_aspects_of(&self, entry_address: &Address) -> Option<Vec<Address>> {
        match self.entry_list.get(entry_address) {
            None => None,
            Some(set) => {
                let vec = set.iter().map(|addr| addr.clone()).collect();
                Some(vec)
            }
        }
    }

    // -- Processing -- //

    /// Add to inbox
    fn post(&mut self, cmd: DhtCommand) -> Lib3hResult<()> {
        self.inbox.push_back(cmd);
        Ok(())
    }

    /// Serve each item in inbox
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)> {
        let now = time::since_epoch_ms();
        let mut outbox = Vec::new();
        // Process inbox
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
                error!("serve_DhtCommand() failed: {:?}", res);
            }
        }
        // Check if others timed-out
        let mut to_remove_list = Vec::new();
        for (peer_address, peer) in self.peer_list.iter() {
            // Skip self
            if peer_address == &self.this_peer.peer_address {
                continue;
            }
            if now - peer.timestamp > self.config.timeout_threshold {
                debug!("@MirrorDht@ peer {} timed-out", peer_address);
                outbox.push(DhtEvent::PeerTimedOut(peer_address.clone()));
                to_remove_list.push(peer_address.clone());
                did_work = true;
            }
        }
        // Remove peer data form local dht
        for peer_address in to_remove_list {
            self.peer_list.remove(&peer_address);
        }
        // Check if must gossip self
        trace!("@MirrorDht@ now: {} ; last_gossip: {} ({})", now, self.last_gossip_of_self, self.config.gossip_interval);
        if now - self.last_gossip_of_self > self.config.gossip_interval {
            self.last_gossip_of_self = now;
            let evt = self.gossip_self(self.get_other_peer_list());
            outbox.push(evt);
            did_work = true;
        }
        // Done
        Ok((did_work, outbox))
    }
}

/// Internals
impl MirrorDht {

    // Get all known peers except self
    fn get_other_peer_list(&self) -> Vec<String> {
        self
            .peer_list
            .iter()
            .filter(|(address, _)| *address != &self.this_peer.peer_address)
            .map(|(address, _)| address.clone())
            .collect()
    }

    // Create gossipTo event of your own PeerData (but not to yourself)
    fn gossip_self(&mut self, peer_address_list: Vec<String>) -> DhtEvent {
        let this_peer = self.this_peer();
        let gossip_this_peer = MirrorGossip::Peer(this_peer.clone());
        let mut buf = Vec::new();
        gossip_this_peer
            .serialize(&mut Serializer::new(&mut buf))
            .unwrap();
        trace!(
            "@MirrorDht@ gossiping self: {:?} | to: {:?}",
            this_peer,
            peer_address_list
        );
        let gossip_evt = GossipToData {
            peer_address_list,
            bundle: buf,
        };
        DhtEvent::GossipTo(gossip_evt)
    }

    /// Return true if new peer or updated peer
    fn add_peer(&mut self, peer_info: &PeerData) -> bool {
        trace!("@MirrorDht@ Adding peer: {:?}", peer_info);
        let maybe_peer = self.peer_list.get_mut(&peer_info.peer_address);
        match maybe_peer {
            None => {
                trace!("@MirrorDht@ Adding peer - OK NEW");
                self.peer_list
                    .insert(peer_info.peer_address.clone(), peer_info.clone());
                true
            }
            Some(mut peer) => {
                if peer_info.timestamp <= peer.timestamp {
                    trace!("@MirrorDht@ Adding peer - BAD");
                    return false;
                }
                trace!(
                    "@MirrorDht@ Adding peer - OK UPDATED: {} > {}",
                    peer_info.timestamp,
                    peer.timestamp
                );
                peer.timestamp = peer_info.timestamp;
                true
            }
        }
    }

    /// Return aspect addresses diff between
    /// known aspects and aspects in the entry argument
    fn diff_aspects(&self, entry: &EntryData) -> HashSet<Address> {
        let aspect_address_set: HashSet<_> = entry
            .aspect_list
            .iter()
            .map(|aspect| aspect.aspect_address.clone())
            .collect();
        let maybe_aspects = self.entry_list.get(&entry.entry_address);
        // Insert arg if first aspects
        if maybe_aspects.is_none() {
            return aspect_address_set;
        }
        // Check if arg has new aspects
        let held_aspects: HashSet<_> = maybe_aspects.unwrap().clone();
        let diff: HashSet<_> = aspect_address_set
            .difference(&held_aspects)
            .map(|item| item.clone())
            .collect();
        diff
    }

    /// Add aspect addresses for an entry in our local storage.
    /// Return true if at least one new aspect address was added.
    fn add_entry_aspects(&mut self, entry: &EntryData) -> bool {
        let diff: HashSet<_> = self.diff_aspects(&entry);
        if diff.len() == 0 {
            return false;
        }
        let maybe_known_aspects = self.entry_list.get(&entry.entry_address);
        let new_aspects: HashSet<_> = match maybe_known_aspects {
            None => diff,
            Some(known_aspects) => known_aspects
                .union(&diff)
                .map(|item| item.clone())
                .collect(),
        };
        self.entry_list
            .insert(entry.entry_address.clone(), new_aspects);
        true
    }

    /// Create GossipTo event for entry to all known peers
    fn gossip_entry(&self, entry: &EntryData) -> DhtEvent {
        let entry_gossip = MirrorGossip::Entry(entry.clone());
        let mut buf = Vec::new();
        entry_gossip
            .serialize(&mut Serializer::new(&mut buf))
            .unwrap();
        let gossip_evt = GossipToData {
            peer_address_list: self.get_other_peer_list(),
            bundle: buf,
        };
        DhtEvent::GossipTo(gossip_evt)
    }

    /// Process a DhtEvent Command, sent by our owner.
    /// Return a list of DhtEvent to owner.
    #[allow(non_snake_case)]
    fn serve_DhtCommand(&mut self, cmd: &DhtCommand) -> Lib3hResult<Vec<DhtEvent>> {
        debug!("@MirrorDht@ serving cmd: {:?}", cmd);
        // Note: use same order as the enum
        match cmd {
            // Received gossip from remote node. Bundle must be a serialized MirrorGossip
            DhtCommand::HandleGossip(msg) => {
                trace!("Deserializer msg.bundle: {:?}", msg.bundle);
                let mut de = Deserializer::new(&msg.bundle[..]);
                let maybe_gossip: Result<MirrorGossip, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                if let Err(e) = maybe_gossip {
                    return Err(format_err!("Failed deserializing gossip: {:?}", e));
                }
                // Handle gossiped data
                match maybe_gossip.unwrap() {
                    MirrorGossip::Entry(entry) => {
                        let diff = self.diff_aspects(&entry);
                        if diff.len() > 0 {
                            return Ok(vec![DhtEvent::HoldEntryRequested(
                                self.this_peer.peer_address.clone(),
                                entry,
                            )]);
                        }
                        return Ok(vec![]);
                    }
                    MirrorGossip::Peer(gossiped_peer) => {
                        let maybe_known_peer = self.get_peer(&gossiped_peer.peer_address);
                        if maybe_known_peer.is_none() {
                            return Ok(vec![DhtEvent::HoldPeerRequested(gossiped_peer)]);
                        }
                        let known_peer = maybe_known_peer.unwrap();
                        // Update Peer timestamp
                        if gossiped_peer.timestamp > known_peer.timestamp {
                            let _ = self.add_peer(&gossiped_peer);
                        }
                        return Ok(vec![]);
                    }
                }
            }
            // Ask owner to respond to self
            DhtCommand::FetchEntry(fetch_entry) => {
                self.pending_fetch_request_list
                    .insert(fetch_entry.msg_id.clone());
                return Ok(vec![DhtEvent::EntryDataRequested(fetch_entry.clone())]);
            }
            // Owner is asking us to hold a peer info
            DhtCommand::HoldPeer(new_peer_data) => {
                // Get peer_list before adding new peer (to use when doing gossipTo)
                let other_peer_address_list = self.get_other_peer_list();
                // Store it
                let received_new_content = self.add_peer(new_peer_data);
                println!("HoldPeer({:?}) in {:?}", new_peer_data, self.peer_list);
                // Bail if peer is known and up to date.
                if !received_new_content {
                    return Ok(vec![]);
                }
                let mut event_list = Vec::new();
                // Gossip to everyone to also hold it
                let peer = self
                    .peer_list
                    .get(&new_peer_data.peer_address)
                    .expect("Should have peer by now");
                let peer_gossip = MirrorGossip::Peer(peer.clone());
                let mut buf = Vec::new();
                peer_gossip
                    .serialize(&mut Serializer::new(&mut buf))
                    .unwrap();
                trace!(
                    "@MirrorDht@ gossiping peer: {:?} to {:?}",
                    peer,
                    other_peer_address_list
                );
                let gossip_evt = GossipToData {
                    peer_address_list: other_peer_address_list,
                    bundle: buf,
                };
                event_list.push(DhtEvent::GossipTo(gossip_evt));

                // Gossip back your own PeerData (but not to yourself)
                if new_peer_data.peer_address != self.this_peer().peer_address {
                    let evt = self.gossip_self(vec![new_peer_data.peer_address.clone()]);
                    event_list.push(evt);
                }
                // Done
                Ok(event_list)
            }
            // Owner is holding some entry. Store its address for bookkeeping.
            // Ask for its data and broadcast it because we want fullsync.
            DhtCommand::HoldEntryAspectAddress(entry) => {
                let received_new_content = self.add_entry_aspects(&entry);
                if !received_new_content {
                    return Ok(vec![]);
                }
                // Use entry_address as request_id
                let address_str = entry.entry_address.clone();
                self.pending_fetch_request_list
                    .insert(address_str.to_string());
                let fetch_entry = FetchDhtEntryData {
                    msg_id: address_str.to_string(),
                    entry_address: entry.entry_address.to_owned(),
                };
                Ok(vec![DhtEvent::EntryDataRequested(fetch_entry)])
            }
            // Owner has some entry and wants it stored on the network
            // Bookkeep address and gossip entry to every known peer.
            DhtCommand::BroadcastEntry(entry) => {
                // Store address
                let received_new_content = self.add_entry_aspects(&entry);
                //// Bail if did not receive new content
                if !received_new_content {
                    return Ok(vec![]);
                }
                let gossip_evt = self.gossip_entry(entry);
                // Done
                Ok(vec![gossip_evt])
            }
            // N/A. Do nothing since this is a monotonic fullsync dht
            DhtCommand::DropEntryAddress(_) => Ok(vec![]),
            // EntryDataResponse:
            //   - From a Publish: Forward response back to self
            //   - From a Hold   : Broadcast entry
            DhtCommand::EntryDataResponse(response) => {
                if !self.pending_fetch_request_list.remove(&response.msg_id) {
                    return Err(format_err!("Received response for an unknown request"));
                }
                // From a Hold if msg_id matches one set in HoldEntryAspectAddress
                let address_str: String = (&response.entry.entry_address).clone().into();
                if address_str == response.msg_id {
                    let gossip_evt = self.gossip_entry(&response.entry);
                    return Ok(vec![gossip_evt]);
                }
                Ok(vec![DhtEvent::FetchEntryResponse(response.clone())])
            }
        }
    }
}
