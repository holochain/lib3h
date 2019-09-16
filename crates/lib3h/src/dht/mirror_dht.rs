use crate::{
    dht::{dht_config::DhtConfig, dht_protocol::*, PeerAddress, PeerAddressRef},
    error::{ErrorKind, Lib3hError, Lib3hResult},
    time,
};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::EntryData, Address, DidWork};
use lib3h_tracing::Lib3hSpan;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use url::Url;

type HasTimedOut = bool;

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
    /// Storage of EntryData with empty aspect content?
    entry_list: HashMap<Address, HashSet<Address>>,
    /// Monotonic Storage of PeerData
    peer_map: HashMap<PeerAddress, PeerData>,
    /// Track if peer timed out
    timed_out_map: HashMap<PeerAddress, HasTimedOut>,
    /// PeerData of this peer
    this_peer: PeerData,
    /// Keep track of last time this peer gossiped self to others
    last_gossip_of_self: u64,
    /// Store Dht config used by this peer
    config: DhtConfig,

    /// ghost stuff
    endpoint_parent: Option<DhtEndpoint>,
    endpoint_self: Detach<DhtEndpointWithContext<()>>,
}

/// Constructors
impl MirrorDht {
    pub fn new(this_peer_address: &PeerAddressRef) -> Box<DhtActor> {
        let dht_config = DhtConfig::new(this_peer_address);
        let dht = Self::new_with_config(&dht_config).expect("Failed creating default MirrorDht");
        dht
    }

    pub fn new_with_config(config: &DhtConfig) -> Lib3hResult<Box<DhtActor>> {
        let timestamp = time::since_epoch_ms();
        let (endpoint_parent, endpoint_self) = create_ghost_channel();

        let this = MirrorDht {
            peer_map: HashMap::new(),
            timed_out_map: HashMap::new(),
            entry_list: HashMap::new(),
            this_peer: PeerData {
                // maybe this will go away
                peer_address: config.this_peer_address().to_owned(),
                peer_uri: Url::parse("none:").unwrap(),
                timestamp,
            },
            last_gossip_of_self: timestamp,
            config: config.clone(),
            endpoint_parent: Some(endpoint_parent),
            endpoint_self: Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix("dht_to_parent_")
                    .build(),
            ),
        };
        Ok(Box::new(this))
    }
}

// was Dht Interface
impl MirrorDht {
    // -- Peer info -- //

    fn get_peer_list(&self) -> Vec<PeerData> {
        self.peer_map.values().map(|v| v.clone()).collect()
    }

    fn get_peer(&self, peer_address: &PeerAddressRef) -> Option<PeerData> {
        let res = self.peer_map.get(peer_address);
        if let Some(pd) = res {
            return Some(pd.clone());
        }
        None
    }

    // -- Entry -- //

    fn get_entry_address_list(&self) -> Vec<Address> {
        self.entry_list.iter().map(|kv| kv.0.clone()).collect()
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

    /// Serve each item in inbox
    fn internal_process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtRequestToParent>)> {
        let now = time::since_epoch_ms();
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Check if others timed-out
        // TODO: Might need to optimize performance as walking a map is expensive
        // see comment: https://github.com/holochain/lib3h/pull/210/#discussion_r304518608
        let mut timed_out_list = Vec::new();
        for (peer_address, peer) in self.peer_map.iter() {
            // Skip self
            if peer_address == &self.this_peer.peer_address {
                continue;
            }
            // Skip already timed out
            let has_timed_out = self
                .timed_out_map
                .get(peer_address)
                .expect("Should always have time_out value for a peer");
            if *has_timed_out {
                continue;
            }
            // Check if timed-out
            if now - peer.timestamp > self.config.timeout_threshold() {
                debug!("@MirrorDht@ peer {} timed-out", peer_address);
                outbox.push(DhtRequestToParent::PeerTimedOut(peer_address.clone()));
                timed_out_list.push(peer_address.clone());
                did_work = true;
            }
        }
        // Mark peers that timed out
        for peer_address in timed_out_list {
            self.timed_out_map.insert(peer_address, true);
        }
        // Check if must gossip self
        trace!(
            "@MirrorDht@ now: {} ; last_gossip: {} ({})",
            now,
            self.last_gossip_of_self,
            self.config.gossip_interval(),
        );
        if now - self.last_gossip_of_self > self.config.gossip_interval() {
            self.last_gossip_of_self = now;
            let gossip_data = self.gossip_self(self.get_other_peer_list());
            if gossip_data.peer_address_list.len() > 0 {
                outbox.push(DhtRequestToParent::GossipTo(gossip_data));
                did_work = true;
            }
        }
        // Done
        Ok((did_work, outbox))
    }
}

/// Internals
impl MirrorDht {
    // Get all known peers except self
    fn get_other_peer_list(&self) -> Vec<PeerAddress> {
        self.peer_map
            .iter()
            .filter(|(address, _)| *address != &self.this_peer.peer_address)
            .map(|(address, _)| address.clone())
            .collect()
    }

    // Create gossipTo event of your own PeerData (but not to yourself)
    fn gossip_self(&mut self, peer_address_list: Vec<PeerAddress>) -> GossipToData {
        let gossip_this_peer = MirrorGossip::Peer(self.this_peer.clone());
        let mut buf = Vec::new();
        gossip_this_peer
            .serialize(&mut Serializer::new(&mut buf))
            .unwrap();
        trace!(
            "@MirrorDht@ gossip_self: {:?} | to: {:?}",
            self.this_peer,
            peer_address_list,
        );
        GossipToData {
            peer_address_list,
            bundle: buf.into(),
        }
    }

    /// Return true if new peer or updated peer
    fn add_peer(&mut self, peer_info: &PeerData) -> bool {
        trace!("@MirrorDht@ Adding peer: {:?}", peer_info);
        let maybe_peer = self.peer_map.get_mut(&peer_info.peer_address);
        match maybe_peer {
            None => {
                debug!("@MirrorDht@ Adding peer - OK NEW");
                self.peer_map
                    .insert(peer_info.peer_address.clone(), peer_info.clone());
                self.timed_out_map
                    .insert(peer_info.peer_address.clone(), false);
                true
            }
            Some(mut peer) => {
                if peer_info.timestamp <= peer.timestamp {
                    debug!("@MirrorDht@ Adding peer - BAD");
                    return false;
                }
                debug!(
                    "@MirrorDht@ Adding peer - OK UPDATED: {} > {}",
                    peer_info.timestamp, peer.timestamp,
                );
                peer.timestamp = peer_info.timestamp;
                if crate::time::since_epoch_ms() - peer.timestamp < self.config.timeout_threshold()
                {
                    self.timed_out_map
                        .insert(peer_info.peer_address.clone(), false);
                }
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
    fn gossip_entry(&self, entry: &EntryData) -> DhtRequestToParent {
        let entry_gossip = MirrorGossip::Entry(entry.clone());
        let mut buf = Vec::new();
        entry_gossip
            .serialize(&mut Serializer::new(&mut buf))
            .unwrap();
        let gossip_evt = GossipToData {
            peer_address_list: self.get_other_peer_list(),
            bundle: buf.into(),
        };
        DhtRequestToParent::GossipTo(gossip_evt)
    }
}

/// Impl DhtActor interface
impl
    GhostActor<
        DhtRequestToParent,
        DhtRequestToParentResponse,
        DhtRequestToChild,
        DhtRequestToChildResponse,
        Lib3hError,
    > for MirrorDht
{
    fn take_parent_endpoint(&mut self) -> Option<DhtEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(&mut ()))?;
        for request in self.endpoint_self.as_mut().drain_messages() {
            //debug!("@MirrorDht@ serving request: {:?}", request);
            self.handle_request_from_parent(request)
                .expect("no ghost errors");
        }
        let (did_work, command_list) = self.internal_process().unwrap(); // FIXME unwrap
        for command in command_list {
            self.endpoint_self.publish(Lib3hSpan::todo(), command)?;
        }
        Ok(did_work.into())
    }
}

impl MirrorDht {
    #[allow(irrefutable_let_patterns)]
    fn handle_request_from_parent(&mut self, mut request: DhtToChildMessage) -> Lib3hResult<()> {
        debug!("@MirrorDht@ serving request: {:?}", request);
        match request.take_message().expect("exists") {
            // Received gossip from remote node. Bundle must be a serialized MirrorGossip
            DhtRequestToChild::HandleGossip(msg) => {
                trace!("DhtRequestToChild::HandleGossip: {:?}", msg);
                let mut de = Deserializer::new(&msg.bundle[..]);
                let maybe_gossip: Result<MirrorGossip, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                if let Err(e) = maybe_gossip {
                    error!("Failed to deserialize gossip.");
                    return Err(Lib3hError::new(ErrorKind::RmpSerdeDecodeError(e)));
                }
                // Handle gossiped data
                match maybe_gossip.unwrap() {
                    MirrorGossip::Entry(entry) => {
                        let diff = self.diff_aspects(&entry);
                        if diff.len() > 0 {
                            self.endpoint_self.publish(
                                Lib3hSpan::todo(),
                                DhtRequestToParent::HoldEntryRequested {
                                    from_peer: self.this_peer.peer_address.clone(),
                                    entry,
                                },
                            )?;
                        }
                    }
                    MirrorGossip::Peer(gossiped_peer) => {
                        let maybe_known_peer = self.get_peer(&gossiped_peer.peer_address);
                        match maybe_known_peer {
                            None => {
                                self.endpoint_self.publish(
                                    Lib3hSpan::todo(),
                                    DhtRequestToParent::HoldPeerRequested(gossiped_peer.clone()),
                                )?;
                            }
                            Some(known_peer) => {
                                // Update Peer timestamp
                                if gossiped_peer.timestamp > known_peer.timestamp {
                                    let _is_new_content = self.add_peer(&gossiped_peer);
                                }
                            }
                        }
                    }
                }
            }

            // Owner is asking us to hold a peer info
            DhtRequestToChild::HoldPeer(new_peer_data) => {
                trace!("DhtRequestToChild::HoldPeer: {:?}", new_peer_data);
                // Get peer_list before adding new peer (to use when doing gossipTo)
                let others_list = self.get_other_peer_list();
                // Store it
                let received_new_content = self.add_peer(&new_peer_data);
                // Bail if peer is known and up to date.
                if !received_new_content {
                    return Ok(());
                }
                // Gossip to everyone to also hold it
                let peer = self
                    .peer_map
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
                    others_list
                );
                let gossip_evt = GossipToData {
                    peer_address_list: others_list,
                    bundle: buf.into(),
                };
                self.endpoint_self
                    .publish(Lib3hSpan::todo(), DhtRequestToParent::GossipTo(gossip_evt))?;

                // Gossip back your own PeerData (but not to yourself)
                if new_peer_data.peer_address != self.this_peer.peer_address {
                    let gossip_data = self.gossip_self(vec![new_peer_data.peer_address.clone()]);
                    if gossip_data.peer_address_list.len() > 0 {
                        self.endpoint_self.publish(
                            Lib3hSpan::todo(),
                            DhtRequestToParent::GossipTo(gossip_data),
                        )?;
                    }
                }
            }

            // Owner is holding some entry. Store its address for bookkeeping.
            // Ask for its data and broadcast it because we want fullsync.
            DhtRequestToChild::HoldEntryAspectAddress(entry) => {
                trace!("DhtRequestToChild::HoldEntryAspectAddress: {:?}", entry);
                let received_new_content = self.add_entry_aspects(&entry);
                if !received_new_content {
                    println!("DhtRequestToChild::HoldEntryAspectAddress: known - skipping");
                    return Ok(());
                }
                // broadcast it by gossiping it to every known peer
                let gossip_evt = self.gossip_entry(&entry);
                self.endpoint_self.publish(Lib3hSpan::todo(), gossip_evt)?;
            }

            // Owner has some entry and wants it stored on the network
            // Bookkeep address and gossip entry to every known peer.
            DhtRequestToChild::BroadcastEntry(entry) => {
                trace!("@MirrorDht@ BroadcastEntry: {:?}", entry);
                // Store address
                let received_new_content = self.add_entry_aspects(&entry);
                //// Bail if did not receive new content
                if !received_new_content {
                    return Ok(());
                }
                let gossip_evt = self.gossip_entry(&entry);
                self.endpoint_self.publish(Lib3hSpan::todo(), gossip_evt)?;
            }

            // N/A. Do nothing since this is a monotonic fullsync dht
            DhtRequestToChild::DropEntryAddress(_) => (),

            DhtRequestToChild::UpdateAdvertise(peer_uri) => {
                trace!("DhtRequestToChild::UpdateAdvertise: {:?}", peer_uri);
                self.this_peer.peer_uri = peer_uri;
            }

            DhtRequestToChild::RequestPeer(peer_address) => {
                trace!("DhtRequestToChild::RequestPeer: {:?}", peer_address);
                let maybe_peer = self.get_peer(&peer_address);
                let payload = Ok(DhtRequestToChildResponse::RequestPeer(maybe_peer));
                request.respond(payload)?;
            }

            DhtRequestToChild::RequestPeerList => {
                let list = self.get_peer_list();
                let payload = Ok(DhtRequestToChildResponse::RequestPeerList(list));
                request.respond(payload)?;
            }

            DhtRequestToChild::RequestThisPeer => {
                let payload = Ok(DhtRequestToChildResponse::RequestThisPeer(
                    self.this_peer.clone(),
                ));
                request.respond(payload)?;
            }

            DhtRequestToChild::RequestEntryAddressList => {
                let list = self.get_entry_address_list();
                let payload = Ok(DhtRequestToChildResponse::RequestEntryAddressList(list));
                request.respond(payload)?;
            }

            DhtRequestToChild::RequestAspectsOf(address) => {
                let maybe_list = self.get_aspects_of(&address);
                let payload = Ok(DhtRequestToChildResponse::RequestAspectsOf(maybe_list));
                request.respond(payload)?;
            }

            // Ask owner to respond to self
            DhtRequestToChild::RequestEntry(entry_address) => {
                trace!("DhtRequestToChild::RequestEntry: {:?}", entry_address);
                self.endpoint_self.request(
                    Lib3hSpan::todo(),
                    DhtRequestToParent::RequestEntry(entry_address),
                    Box::new(|_me, response| {
                        let response = {
                            match response {
                                GhostCallbackData::Timeout => panic!("timeout"),
                                GhostCallbackData::Response(response) => match response {
                                    Err(e) => panic!("{:?}", e),
                                    Ok(response) => response,
                                },
                            }
                        };
                        if let DhtRequestToParentResponse::RequestEntry(entry_response) = response {
                            println!("4. In DhtRequestToChild::RequestEntry Responding...");
                            let payload =
                                Ok(DhtRequestToChildResponse::RequestEntry(entry_response));
                            request.respond(payload)?;
                        } else {
                            panic!("bad response to RequestEntry: {:?}", response);
                        }
                        Ok(())
                    }),
                )?;
            }
        };
        Ok(())
    }
}
