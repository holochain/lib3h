use crate::dht::PeerAddress;
use lib3h_protocol::{data_types::{EntryData, Opaque}, Address};
use url::Url;

pub type FromPeerAddress = PeerAddress;

#[derive(Debug, PartialEq, Clone)]
pub enum DhtCommand {
    /// Owner received a gossip bundle from a remote peer, and asks us to handle it.
    HandleGossip(RemoteGossipBundleData),
    /// Owner wants a specific entry.
    FetchEntry(FetchDhtEntryData),
    /// Owner wants us to hold a peer discovery data item.
    HoldPeer(PeerData),
    /// Owner notifies us that it is holding one or several Aspects for an Entry.
    /// Note: Need an EntryData to know the aspect addresses, but aspects' content can be empty.
    HoldEntryAspectAddress(EntryData),
    /// Owner wants us to bookkeep an entry and broadcast it to neighbors
    BroadcastEntry(EntryData),
    /// Owner notifies us that is is not holding an entry anymore.
    DropEntryAddress(Address),
    /// Owner's response to ProvideEntry request
    EntryDataResponse(FetchDhtEntryResponseData),
}

#[derive(Debug, PartialEq, Clone)]
pub enum DhtEvent {
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
    HoldEntryRequested(FromPeerAddress, EntryData),
    /// DHT wants an entry in order to send it to someone on the network
    EntryDataRequested(FetchDhtEntryData),
    /// Response to a `FetchEntry` command.
    FetchEntryResponse(FetchDhtEntryResponseData),
    /// Notify owner that we are no longer tracking this entry internally.
    /// Owner should purge this address from storage, but they can, of course, choose not to.
    EntryPruned(Address),
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct RemoteGossipBundleData {
    pub from_peer_address: PeerAddress,
    pub bundle: Opaque,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct GossipToData {
    pub peer_address_list: Vec<PeerAddress>,
    pub bundle: Opaque,
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
