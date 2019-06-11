use lib3h_protocol::{data_types::EntryData, Address};

#[derive(Debug, PartialEq, Clone)]
pub enum DhtEvent {
    /// We have received a gossip bundle from a remote peer,
    /// pass it along to the dht backend for processing
    RemoteGossipBundle(RemoteGossipBundleData),
    /// Instructs implementors to send this binary gossip bundle
    /// to the specified list of peerAddress' in a reliable manner.
    GossipTo(GossipToData),
    /// Instructs implementors to send this binary gossip bundle
    /// to as many peers listed in peerList as possible.
    /// It is okay if not all peers on the list receive the message.
    UnreliableGossipTo(GossipToData),
    /// Tell implementors that gossip is requesting we hold a peer discovery
    /// data item. Note that this dht tracker has not actually marked this item
    /// for holding until the implementors pass this event back in.
    PeerHoldRequest(PeerHoldRequestData),
    /// Tell implementors that gossip believes a peer has dropped
    PeerTimedOut(String),
    /// Tell implementors that gossip is requesting we hold an entry.
    /// Note that this dht tracker has not actually marked this item
    /// for holding until the implementors pass this event back in.
    EntryHoldRequest(EntryData),
    /// This dht tracker requires access to the entry associated with a entry address.
    /// This event should cause implementors to respond with a dataFetchResponse
    /// event.
    EntryFetch(EntryFetchData),
    /// Response to a `EntryFetch` event. Set `entry` to `null` to indicate the
    /// requested entry is not available (it will be removed from gossip).
    EntryFetchResponse(EntryFetchResponseData),
    /// Tell our implementors that we are no longer tracking this entry
    /// locally. Implementors should purge this address from storage,
    /// but that can, of course, choose not to.
    EntryPrune(Address),
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
pub struct PeerHoldRequestData {
    pub peer_address: String,
    pub transport: String,
    pub timestamp: u64,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct EntryFetchData {
    pub msg_id: String,
    pub entry_address: Address,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct EntryFetchResponseData {
    pub msg_id: String,
    pub entry: EntryData,
}
