use crate::dht::PeerAddress;
use lib3h_protocol::{data_types::EntryData, Address};
use url::Url;

use crate::{dht::dht_config::DhtConfig, error::*};
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::data_types::*;

pub type FromPeerAddress = PeerAddress;

pub type DhtFactory = fn(config: &DhtConfig) -> Lib3hResult<Box<DhtActor>>;

pub type DhtActor = dyn GhostActor<
    DhtRequestToParent,
    DhtRequestToParentResponse,
    DhtRequestToChild,
    DhtRequestToChildResponse,
    Lib3hError,
>;
pub type DhtEndpointWithContext<UserData> = GhostContextEndpoint<
    UserData,
    DhtContext,
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
    DhtContext,
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

#[derive(Debug)]
pub enum DhtContext {
    NoOp,
    RequestAspectsOf {
        entry_address: Address,
        aspect_address_list: Vec<Address>,
        msg: EntryListData,
        request_id: String,
    },
    RequestEntry(DhtToChildMessage),
    QueryEntry(QueryEntryData),
}

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
    DropEntryAddress(Address),

    /// Requests
    /// Parent wants PeerData for a specific Peer
    RequestPeer(String),
    /// Parent wants the list of peers we are holding
    RequestPeerList,
    /// Parent wants PeerData of this entity
    RequestThisPeer,
    /// Parent wants the list of entries we are holding
    RequestEntryAddressList,
    /// Parent wants address' we have for an entry
    RequestAspectsOf(Address),
    /// Parent wants a specific entry.
    RequestEntry(Address),
}

#[derive(Debug, Clone)]
pub enum DhtRequestToChildResponse {
    RequestPeer(Option<PeerData>),
    RequestPeerList(Vec<PeerData>),
    RequestThisPeer(PeerData),
    RequestEntryAddressList(Vec<Address>),
    RequestAspectsOf(Option<Vec<Address>>),
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
    PeerTimedOut(PeerAddress),
    /// Notify owner that gossip is requesting we hold an entry.
    HoldEntryRequested { from_peer: String, entry: EntryData },
    /// Notify owner that we are no longer tracking this entry internally.
    /// Owner should purge this address from storage, but they can, of course, choose not to.
    EntryPruned(Address),

    /// Requests
    /// DHT wants an entry in order to send it to someone on the network
    RequestEntry(Address),
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
    pub from_peer_address: PeerAddress,
    pub bundle: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct GossipToData {
    pub peer_address_list: Vec<PeerAddress>,
    pub bundle: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct PeerData {
    pub peer_address: PeerAddress,
    #[serde(with = "url_serde")]
    pub peer_uri: Url,
    pub timestamp: u64,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct FetchDhtEntryData {
    pub msg_id: String,
    pub entry_address: Address,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct FetchDhtEntryResponseData {
    pub msg_id: String,
    pub entry: EntryData,
}
