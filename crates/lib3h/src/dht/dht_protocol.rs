use lib3h_protocol::{
    data_types::{EntryData, Opaque},
    types::*,
    uri::Lib3hUri,
};

use crate::{dht::dht_config::DhtConfig, error::*};
use lib3h_ghost_actor::prelude::*;

pub type FromPeerName = Lib3hUri;

pub type DhtFactory =
    fn(config: &DhtConfig, maybe_this_peer: Option<PeerData>) -> Lib3hResult<Box<DhtActor>>;

pub type DhtActor = dyn GhostActor<
    DhtRequestToParent,
    DhtRequestToParentResponse,
    DhtRequestToChild,
    DhtRequestToChildResponse,
    Lib3hError,
>;
pub type DhtEndpointWithContext<UserData> = GhostContextEndpoint<
    UserData,
    DhtRequestToParent,
    DhtRequestToParentResponse,
    DhtRequestToChild,
    DhtRequestToChildResponse,
    Lib3hError,
>;
pub type DhtEndpoint = GhostEndpoint<
    DhtRequestToChild,
    DhtRequestToChildResponse,
    DhtRequestToParent,
    DhtRequestToParentResponse,
    Lib3hError,
>;
pub type ChildDhtWrapperDyn<UserData> = GhostParentWrapperDyn<
    UserData,
    DhtRequestToParent,
    DhtRequestToParentResponse,
    DhtRequestToChild,
    DhtRequestToChildResponse,
    Lib3hError,
>;

pub type DhtToChildMessage =
    GhostMessage<DhtRequestToChild, DhtRequestToParent, DhtRequestToChildResponse, Lib3hError>;

pub type DhtToParentMessage =
    GhostMessage<DhtRequestToParent, DhtRequestToChild, DhtRequestToParentResponse, Lib3hError>;

#[derive(Debug, Clone)]
pub enum DhtRequestToChild {
    /// Commands
    /// Parent received a gossip bundle from a remote peer, and asks us to handle it.
    HandleGossip(RemoteGossipBundleData),
    /// Parent wants us to hold a peer discovery data item.
    HoldPeer(PeerData),
    /// Parent notifies us that it is holding one or several Aspects for an Entry.
    /// Note: Need an EntryData to know the aspect addresses, but aspects' content can be empty.
    HoldEntryAspectAddress(EntryData),
    /// Parent wants us to bookkeep an entry and broadcast it to neighbors
    BroadcastEntry(EntryData),
    /// Parent notifies us that is is not holding an entry anymore.
    DropEntryAddress(EntryHash),

    /// Parent notifies us that the binding changed
    UpdateAdvertise(Lib3hUri),

    /// Requests
    /// Parent wants PeerData for a specific Peer
    RequestPeer(Lib3hUri),
    /// Parent wants the list of peers we are holding
    RequestPeerList,
    /// Parent wants PeerData of this entity
    RequestThisPeer,
    /// Parent wants the list of entries we are holding
    RequestEntryAddressList,
    /// Parent wants address' we have for an entry
    RequestAspectsOf(EntryHash),
    /// Parent wants a specific entry.
    RequestEntry(EntryHash),
}

#[derive(Debug, Clone)]
pub enum DhtRequestToChildResponse {
    RequestPeer(Option<PeerData>),
    RequestPeerList(Vec<PeerData>),
    RequestThisPeer(PeerData),
    RequestEntryAddressList(Vec<EntryHash>),
    RequestAspectsOf(Option<Vec<AspectHash>>),
    RequestEntry(EntryData),
}

#[derive(Debug, Clone)]
pub enum DhtRequestToParent {
    /// Commands & Events
    /// Ask owner to send this binary gossip bundle
    /// to the specified list of peerAddress' in a reliable manner.
    GossipTo(GossipToData),
    /// Ask owner to send this binary gossip bundle
    /// to as many peers listed in peerList as possible.
    /// It is okay if not all peers on the list receive the message.
    GossipUnreliablyTo(GossipToData),
    /// Notify owner that gossip is requesting we hold a peer discovery data item.
    HoldPeerRequested(PeerData),
    /// Notify owner that we believe a peer has dropped
    PeerTimedOut(Lib3hUri),
    /// Notify owner that gossip is requesting we hold an entry.
    HoldEntryRequested {
        from_peer_name: Lib3hUri,
        entry: EntryData,
    },
    /// Notify owner that we are no longer tracking this entry internally.
    /// Owner should purge this address from storage, but they can, of course, choose not to.
    EntryPruned(EntryHash),

    /// Requests
    /// DHT wants an entry in order to send it to someone on the network
    RequestEntry(EntryHash),
}

#[derive(Debug, Clone)]
pub enum DhtRequestToParentResponse {
    RequestEntry(EntryData),
}

//--------------------------------------------------------------------------------------------------
// Data types
//--------------------------------------------------------------------------------------------------

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct RemoteGossipBundleData {
    pub from_peer_name: Lib3hUri,
    pub bundle: Opaque,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct GossipToData {
    pub peer_name_list: Vec<Lib3hUri>,
    pub bundle: Opaque,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct PeerData {
    pub peer_name: Lib3hUri,
    pub peer_location: Lib3hUri,
    pub timestamp: u64,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct FetchDhtEntryData {
    pub msg_id: String,
    pub entry_address: EntryHash,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct FetchDhtEntryResponseData {
    pub msg_id: String,
    pub entry: EntryData,
}
