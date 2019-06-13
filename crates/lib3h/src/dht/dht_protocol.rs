use lib3h_protocol::{data_types::EntryData, Address};

#[derive(Debug, PartialEq, Clone)]
pub enum DhtCommand {
    /// Owner received a gossip bundle from a remote peer, and asks us to handle it.
    HandleGossip(RemoteGossipBundleData),
    /// Owner wants access to the entry associated with an entry address.
    FetchEntry(FetchEntryData),
    /// Owner wants us to hold a peer discovery data item.
    HoldPeer(PeerData),
    /// Owner wants us to hold an entry.
    HoldEntry(EntryData),
    /// Owner wants us to hold an entry and broadcast it to neighbors
    BroadcastEntry(EntryData),
    /// Owner wants us to drop an entry.
    DropEntry(Address),
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
    PeerTimedOut(String),
    /// Notify owner that gossip is requesting we hold an entry.
    HoldEntryRequested(EntryData),
    /// Response to a `FetchEntry` command.
    FetchEntryResponse(FetchEntryResponseData),
    /// Notify owner that we are no longer tracking this entry internally.
    /// Owner should purge this address from storage, but they can, of course, choose not to.
    EntryPruned(Address),
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct RemoteGossipBundleData {
    pub from_peer_address: String,
    pub bundle: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct GossipToData {
    pub peer_address_list: Vec<String>,
    pub bundle: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct PeerData {
    pub peer_address: String,
    pub transport: String,
    pub timestamp: u64,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct FetchEntryData {
    pub msg_id: String,
    pub entry_address: Address,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct FetchEntryResponseData {
    pub msg_id: String,
    pub entry: EntryData,
}
