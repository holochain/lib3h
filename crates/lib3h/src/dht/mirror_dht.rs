use crate::{
    dht::{dht_config::DhtConfig, dht_protocol::*},
    error::{ErrorKind, Lib3hError, Lib3hResult},
    time,
};
use detach::prelude::*;
use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::EntryData, types::*, uri::Lib3hUri, DidWork};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
    entry_list: HashMap<EntryHash, HashSet<AspectHash>>,
    /// Monotonic Storage of PeerData
    peer_map: HashMap<Lib3hUri, PeerData>,
    /// Track if peer timed out
    timed_out_map: HashMap<Lib3hUri, HasTimedOut>,
    /// PeerData of this peer
    this_peer: PeerData,
    /// Keep track of last time this peer gossiped self to others
    last_gossip_of_self: u64,
    /// Store Dht config used by this peer
    config: DhtConfig,

    /// ghost stuff
    endpoint_parent: Option<DhtEndpoint>,
    endpoint_self: Detach<DhtEndpointWithContext<Self>>,
}

/// Constructors
impl MirrorDht {
    pub fn with_this_peer(this_peer: &PeerData) -> Box<DhtActor> {
        let dht_config = DhtConfig::new(&this_peer.peer_name);
        let dht = Self::new_with_config(&dht_config, Some(this_peer.clone()))
            .expect("Failed creating default MirrorDht");
        dht
    }

    pub fn new(this_peer_name: &Lib3hUri) -> Box<DhtActor> {
        let dht_config = DhtConfig::new(this_peer_name);
        let dht =
            Self::new_with_config(&dht_config, None).expect("Failed creating default MirrorDht");
        dht
    }

    pub fn new_with_config(
        config: &DhtConfig,
        maybe_this_peer: Option<PeerData>,
    ) -> Lib3hResult<Box<DhtActor>> {
        let timestamp = time::since_epoch_ms();
        let (endpoint_parent, endpoint_self) = create_ghost_channel();

        let this_peer = match maybe_this_peer {
            None => PeerData {
                peer_name: config.this_peer_name().to_owned(),
                peer_location: Lib3hUri::with_undefined(),
                timestamp,
            },
            Some(this_peer) => this_peer,
        };

        let this = MirrorDht {
            peer_map: HashMap::new(),
            timed_out_map: HashMap::new(),
            entry_list: HashMap::new(),
            this_peer,
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

    fn get_peer(&self, peer_name: &Lib3hUri) -> Option<PeerData> {
        if peer_name == &self.this_peer.peer_name {
            return Some(self.this_peer.clone());
        }
        let res = self.peer_map.get(peer_name);
        if let Some(pd) = res {
            return Some(pd.clone());
        }
        None
    }

    // -- Entry -- //

    fn get_entry_address_list(&self) -> Vec<EntryHash> {
        self.entry_list.iter().map(|kv| kv.0.clone()).collect()
    }

    fn get_aspects_of(&self, entry_address: &EntryHash) -> Option<Vec<AspectHash>> {
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
        for (peer_name, peer) in self.peer_map.iter() {
            // Skip self
            if peer_name == &self.this_peer.peer_name {
                continue;
            }
            // Skip already timed out
            let has_timed_out = self
                .timed_out_map
                .get(peer_name)
                .expect("Should always have time_out value for a peer");
            if *has_timed_out {
                continue;
            }
            // Check if timed-out
            if now - peer.timestamp > self.config.timeout_threshold() {
                debug!(
                    "@MirrorDht@ peer {} timed-out ({} > {})",
                    peer_name,
                    now - peer.timestamp,
                    self.config.timeout_threshold()
                );
                outbox.push(DhtRequestToParent::PeerTimedOut(peer_name.clone()));
                timed_out_list.push(peer_name.clone());
                did_work = true;
            }
        }
        // Mark peers that timed out
        for peer_name in timed_out_list {
            self.timed_out_map.insert(peer_name, true);
        }
        // Check if must gossip self
        /*trace!(
            "@MirrorDht@ now: {} ; last_gossip: {} ({})",
            now,
            self.last_gossip_of_self,
            self.config.gossip_interval(),
        );*/
        if now - self.last_gossip_of_self > self.config.gossip_interval() {
            self.last_gossip_of_self = now;
            let gossip_data = self.gossip_self(self.get_other_peer_list());
            if gossip_data.peer_name_list.len() > 0 {
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
    fn get_other_peer_list(&self) -> Vec<Lib3hUri> {
        self.peer_map
            .iter()
            .filter(|(address, _)| *address != &self.this_peer.peer_name)
            .map(|(address, _)| address.clone())
            .collect()
    }

    // Create gossipTo event of your own PeerData (but not to yourself)
    fn gossip_self(&mut self, peer_name_list: Vec<Lib3hUri>) -> GossipToData {
        let gossip_this_peer = MirrorGossip::Peer(self.this_peer.clone());
        let mut buf = Vec::new();
        gossip_this_peer
            .serialize(&mut Serializer::new(&mut buf))
            .unwrap();
        //        trace!(
        //            "@MirrorDht@ gossip_self: {:?} | to: {:?}",
        //            self.this_peer,
        //            peer_name_list,
        //        );
        GossipToData {
            peer_name_list,
            bundle: buf.into(),
        }
    }

    /// Return true if new peer or updated peer
    fn add_peer(&mut self, peer_info: &PeerData) -> bool {
        debug!(
            "@MirrorDht@ {:?} Adding peer: {:?}",
            self.this_peer, peer_info
        );
        let maybe_peer = self.peer_map.get_mut(&peer_info.peer_name);
        match maybe_peer {
            None => {
                debug!("@MirrorDht@ Adding peer - OK NEW");
                self.peer_map
                    .insert(peer_info.peer_name.clone(), peer_info.clone());
                self.timed_out_map
                    .insert(peer_info.peer_name.clone(), false);
                true
            }
            Some(mut peer) => {
                if peer_info.timestamp < peer.timestamp {
                    debug!(
                        "@MirrorDht@ Adding peer - BAD {:?} has earlier timestamp than {:?}",
                        peer_info.timestamp, peer.timestamp
                    );
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
                        .insert(peer_info.peer_name.clone(), false);
                }
                true
            }
        }
    }

    /// Return aspect addresses diff between
    /// known aspects and aspects in the entry argument
    fn diff_aspects(&self, entry: &EntryData) -> HashSet<AspectHash> {
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
            peer_name_list: self.get_other_peer_list(),
            bundle: buf.into(),
        };
        debug!(
            "@MirrorDht@ {:?} GossipTo: {:?}",
            self.this_peer, gossip_evt,
        );
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
        detach_run!(&mut self.endpoint_self, |es| es.process(self))?;
        for request in self.endpoint_self.as_mut().drain_messages() {
            //debug!("@MirrorDht@ serving request: {:?}", request);
            self.handle_request_from_parent(request)
                .expect("no ghost errors");
        }
        let (did_work, command_list) = self.internal_process().unwrap(); // FIXME unwrap
        for command in command_list {
            self.endpoint_self
                .publish(Span::todo("where does span come from?"), command)?;
        }
        Ok(did_work.into())
    }
}

impl MirrorDht {
    #[allow(irrefutable_let_patterns)]
    fn handle_request_from_parent(&mut self, mut request: DhtToChildMessage) -> Lib3hResult<()> {
        //debug!("@MirrorDht@ serving request: {:?}", request);
        let span = request.span().child("handle_request_from_parent");
        let msg = request.take_message().expect("exists");
        //debug!("@MirrorDht@ - request: {:?}", msg);
        match msg {
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
                        trace!("DhtRequestToChild::HandleGossip: Entry = {:?}", entry);
                        let diff = self.diff_aspects(&entry);
                        if diff.len() > 0 {
                            self.endpoint_self.publish(
                                span,
                                DhtRequestToParent::HoldEntryRequested {
                                    from_peer_name: self.this_peer.peer_name.clone(),
                                    entry,
                                },
                            )?;
                        }
                    }
                    MirrorGossip::Peer(gossiped_peer) => {
                        trace!(
                            "DhtRequestToChild::HandleGossip: Peer = {:?}",
                            gossiped_peer
                        );
                        let maybe_known_peer = self.get_peer(&gossiped_peer.peer_name);
                        match maybe_known_peer {
                            None => {
                                self.endpoint_self.publish(
                                    span,
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
                let span_hold = span.child("handle DhtRequestToChild::HoldPeer");
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
                    .get(&new_peer_data.peer_name)
                    .expect("Should have peer by now");
                let peer_gossip = MirrorGossip::Peer(peer.clone());
                let mut buf = Vec::new();
                peer_gossip
                    .serialize(&mut Serializer::new(&mut buf))
                    .unwrap();
                trace!(
                    "@MirrorDht@ gossiping peer: {:?} to {:?}",
                    peer,
                    others_list,
                );
                let gossip_evt = GossipToData {
                    peer_name_list: others_list,
                    bundle: buf.into(),
                };

                self.endpoint_self.publish(
                    span_hold
                        .child("send event DhtRequestToParent::GossipTo all the received PeerData"),
                    DhtRequestToParent::GossipTo(gossip_evt),
                )?;

                // Gossip back your own PeerData (but not to yourself)
                if new_peer_data.peer_name != self.this_peer.peer_name {
                    let gossip_data = self.gossip_self(vec![new_peer_data.peer_name.clone()]);
                    if gossip_data.peer_name_list.len() > 0 {
                        self.endpoint_self.publish(
                            span_hold.child(
                                "send event DhtRequestToParent::GossipTo all our own PeerData",
                            ),
                            DhtRequestToParent::GossipTo(gossip_data),
                        )?;
                    }
                }
            }

            // Owner is holding some entry. Store its address for bookkeeping.
            // Ask for its data and broadcast it because we want fullsync.
            DhtRequestToChild::HoldEntryAspectAddress(entry) => {
                trace!(
                    "({:?}).DhtRequestToChild::HoldEntryAspectAddress: {:?}",
                    self.config.this_peer_name(),
                    entry
                );
                // if its shallow, ask for actual data
                if entry.aspect_list.len() > 0 && entry.aspect_list[0].aspect.len() == 0 {
                    self.endpoint_self.publish(
                        span.follower("DhtRequestToChild::HoldEntryAspectAddress"),
                        DhtRequestToParent::RequestEntry(entry.entry_address.clone()),
                    )?;
                    return Ok(());
                }
                // Check if all aspects for this entry are already known
                let received_new_content = self.add_entry_aspects(&entry);
                if !received_new_content {
                    trace!("DhtRequestToChild::HoldEntryAspectAddress: known - skipping");
                    return Ok(());
                }
                // broadcast it by gossiping it to every known peer
                let gossip_evt = self.gossip_entry(&entry);
                self.endpoint_self.publish(
                    span.follower("DhtRequestToChild::HoldEntryAspectAddress"),
                    gossip_evt,
                )?;
            }

            // Owner has some entry and wants it stored on the network
            // Bookkeep address and gossip entry to every known peer.
            DhtRequestToChild::BroadcastEntry(entry) => {
                trace!("@MirrorDht@ BroadcastEntry: {:?}", entry);
                // Store address
                let received_new_content = self.add_entry_aspects(&entry);
                //// Bail if did not receive new content
                if !received_new_content {
                    trace!(
                        "@MirrorDht@ did not receive new content from entry {:?}",
                        entry
                    );
                    return Ok(());
                }
                let gossip_evt = self.gossip_entry(&entry);
                self.endpoint_self.publish(
                    span.follower("DhtRequestToChild::BroadcastEntry"),
                    gossip_evt,
                )?;
            }

            // N/A. Do nothing since this is a monotonic fullsync dht
            DhtRequestToChild::DropEntryAddress(_) => (),

            DhtRequestToChild::UpdateAdvertise(peer_location) => {
                trace!(
                    "({}).DhtRequestToChild::UpdateAdvertise: {:?}",
                    self.config.this_peer_name(),
                    peer_location
                );
                self.this_peer.peer_location = peer_location;
            }

            DhtRequestToChild::RequestPeer(peer_name) => {
                trace!("DhtRequestToChild::RequestPeer: {:?}", peer_name);
                let maybe_peer = self.get_peer(&peer_name);
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
                //                trace!(
                //                    "DhtRequestToChild::RequestThisPeer:  sending {:?}",
                //                    self.this_peer
                //                );
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
                    span.child("DhtRequestToChild::RequestEntry"),
                    DhtRequestToParent::RequestEntry(entry_address),
                    Box::new(|_me, response| {
                        let response = {
                            match response {
                                GhostCallbackData::Timeout(bt) => panic!("timeout: {:?}", bt),
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
