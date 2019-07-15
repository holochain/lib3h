use crate::{
    dht::{
        dht_protocol::*,
        dht_trait::{Dht, DhtConfig},
    },
    error::{ErrorKind, Lib3hError, Lib3hResult},
};
use lib3h_protocol::{data_types::EntryData, Address, DidWork};
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
                peer_uri: peer_transport.clone(),
                timestamp: 0, // FIXME
            },
            pending_fetch_request_list: HashSet::new(),
        }
    }

    pub fn new_with_config(config: &DhtConfig) -> Lib3hResult<Self> {
        Ok(Self::new(&config.this_peer_address, &config.this_peer_uri))
    }
}

/// Impl Dht interface
impl Dht for MirrorDht {
    // -- Peer info -- //

    fn get_peer_list(&self) -> Vec<PeerData> {
        self.peer_list.values().map(|v| v.clone()).collect()
    }

    fn get_peer(&self, peer_address: &str) -> Option<PeerData> {
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
                error!("serve_DhtCommand() failed: {:?}", res);
            }
        }
        Ok((did_work, outbox))
    }
}

/// Internals
impl MirrorDht {
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
                trace!("@MirrorDht@ Adding peer - OK UPDATED");
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
            peer_address_list: self
                .get_peer_list()
                .iter()
                .map(|pi| pi.peer_address.clone())
                .collect(),
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
                    error!("Fail to deserialize gossip.");
                    return Err(Lib3hError::new(ErrorKind::RmpSerdeDecodeError(e)));
                    // return Err(e.into());
                }
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
                    MirrorGossip::Peer(peer) => {
                        let is_new = self.add_peer(&peer);
                        if is_new {
                            return Ok(vec![DhtEvent::HoldPeerRequested(peer)]);
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
                trace!("@MirrorDht@ gossiping peer: {:?}", peer);
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
                    trace!(
                        "@MirrorDht@ gossiping peer back: {:?} | to: {}",
                        peer,
                        msg.peer_address
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
                    return Err(Lib3hError::new(ErrorKind::Other(String::from(
                        "Received response for an unknown request",
                    ))));
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
