#![feature(try_from)]

//! This module provides the api definition for working with lib3h

#[macro_use]
extern crate serde_derive;

/// Opaque Address Bytes
pub type Address = Vec<u8>;

/// Tuple holding all the info required for identifying a metadata.
/// (entry_address, attribute, content)
pub type MetaTuple = (Address, String, Vec<u8>);
/// (entry_address, attribute)
pub type MetaKey = (Address, String);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StateData {
    pub state: String,
    pub id: String,
    pub bindings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ConfigData {
    pub config: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ConnectData {
    pub peer_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PeerData {
    pub agent_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MessageData {
    pub dna_address: Address,
    pub request_id: String,
    pub to_agent_id: String,
    pub from_agent_id: String,
    pub content: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TrackDnaData {
    pub dna_address: Address,
    pub agent_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SuccessResultData {
    pub dna_address: Address,
    pub request_id: String,
    pub to_agent_id: String,
    pub success_info: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FailureResultData {
    pub dna_address: Address,
    pub request_id: String,
    pub to_agent_id: String,
    pub error_info: Vec<u8>,
}

//--------------------------------------------------------------------------------------------------
// DHT Entry
//--------------------------------------------------------------------------------------------------

/// Drop some data request from own p2p-module
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DropEntryData {
    pub dna_address: Address,
    pub request_id: String,
    pub entry_address: Address,
}

/// Data Request from some other agent
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FetchEntryData {
    pub dna_address: Address,
    pub request_id: String,
    pub requester_agent_id: String,
    pub entry_address: Address,
}

/// Generic DHT data message
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct EntryData {
    pub dna_address: Address,
    pub provider_agent_id: String,
    pub entry_address: Address,
    pub entry_content: Vec<u8>,
}

/// DHT data response from a request
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct FetchEntryResultData {
    pub dna_address: Address,
    pub request_id: String,
    pub requester_agent_id: String,
    pub provider_agent_id: String,
    pub entry_address: Address,
    pub entry_content: Vec<u8>,
}

//--------------------------------------------------------------------------------------------------
// DHT metadata
//--------------------------------------------------------------------------------------------------

/// Metadata Request from another agent
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FetchMetaData {
    pub dna_address: Address,
    pub request_id: String,
    pub requester_agent_id: String,
    pub entry_address: Address,
    pub attribute: String,
}

/// Generic DHT metadata message
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DhtMetaData {
    pub dna_address: Address,
    pub provider_agent_id: String,
    pub entry_address: Address,
    pub attribute: String,
    // single string or list of hashs
    pub content_list: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FetchMetaResultData {
    pub dna_address: Address,
    pub request_id: String,
    pub requester_agent_id: String,
    pub provider_agent_id: String,
    pub entry_address: Address,
    pub attribute: String,
    // // List of (hash, content) pairs.
    // single string or list of hashs
    pub content_list: Vec<Vec<u8>>,
}

/// Drop some data request from own p2p-module
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DropMetaData {
    pub dna_address: Address,
    pub request_id: String,
    pub entry_address: Address,
    pub attribute: String,
}

//--------------------------------------------------------------------------------------------------
// List (publish & hold)
//--------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GetListData {
    pub dna_address: Address,
    pub request_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EntryListData {
    pub dna_address: Address,
    pub request_id: String,
    pub entry_address_list: Vec<Address>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MetaListData {
    pub dna_address: Address,
    pub request_id: String,
    // List of meta identifiers, a pair: (entry_address, attribute, hashed_content)
    pub meta_list: Vec<MetaTuple>,
}

//--------------------------------------------------------------------------------------------------
// Lib3hProtocol Enum
//--------------------------------------------------------------------------------------------------

/// Enum holding all message types that serialize as json in the 'hc-core <-> P2P network module' protocol.
/// There are 4 categories of messages:
///  - Command: An order from the local node to the p2p module. Local node expects a reponse. Starts with a verb.
///  - Handle-command: An order from the p2p module to the local node. The p2p module expects a response. Start withs 'Handle' followed by a verb.
///  - Result: A response to a Command. Starts with the name of the Command it responds to and ends with 'Result'.
///  - Notification: Notify that something happened. Not expecting any response. Ends with verb in past form, i.e. '-ed'.
/// Fetch = Request between node and the network (other nodes)
/// Get   = Request within a node between p2p module and core
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Lib3hProtocol {
    // -- Generic responses -- //
    /// Success response to a request (any message with an _id field.)
    SuccessResult(SuccessResultData),
    /// Failure response to a request (any message with an _id field.)
    /// Can also be a response to a mal-formed request.
    FailureResult(FailureResultData),

    // -- Connection -- //
    /// Order the p2p module to be part of the network of the specified DNA.
    TrackDna(TrackDnaData),

    /// Order the p2p module to leave the network of the specified DNA.
    UntrackDna(TrackDnaData),

    /// Connect to the specified multiaddr
    Connect(ConnectData),
    /// Notification of a connection from another peer.
    PeerConnected(PeerData),

    // -- Config (deprecated?) -- //
    /// Request the current state from the p2p module
    GetState,
    /// p2p module's current state response.
    GetStateResult(StateData),
    /// Request the default config from the p2p module
    GetDefaultConfig,
    /// the p2p module's default config response
    GetDefaultConfigResult(ConfigData),
    /// Set the p2p config
    SetConfig(ConfigData),

    // -- Direct Messaging -- //
    /// Send a message to another peer on the network
    SendMessage(MessageData),
    /// the response from a previous `SendMessage`
    SendMessageResult(MessageData),
    /// Request to handle a message another peer has sent us.
    HandleSendMessage(MessageData),
    /// Our response to a message from another peer.
    HandleSendMessageResult(MessageData),

    // -- Entry -- //
    /// Request data from the dht network
    FetchEntry(FetchEntryData),
    /// Response from requesting dht data from the network
    FetchEntryResult(FetchEntryResultData),
    /// Another node, or the network module itself is requesting data from us
    HandleFetchEntry(FetchEntryData),
    /// Successful data response for a `HandleFetchDhtData` request
    HandleFetchEntryResult(FetchEntryResultData),

    /// Publish data to the dht.
    PublishEntry(EntryData),
    /// Store data on a node's dht slice.
    HandleStoreEntry(EntryData),
    HandleDropEntry(DropEntryData),

    // -- Meta -- //
    /// Request metadata from the dht
    FetchMeta(FetchMetaData),
    /// Response by the network for our metadata request
    FetchMetaResult(FetchMetaResultData),
    /// Another node, or the network module itself, is requesting data from us
    HandleFetchMeta(FetchMetaData),
    /// Successful metadata response for a `HandleFetchMeta` request
    HandleFetchMetaResult(FetchMetaResultData),

    /// Publish metadata to the dht.
    PublishMeta(DhtMetaData),
    /// Store metadata on a node's dht slice.
    HandleStoreMeta(DhtMetaData),
    /// Drop metadata on a node's dht slice.
    HandleDropMeta(DropMetaData),

    // -- Entry lists -- //
    HandleGetPublishingEntryList(GetListData),
    HandleGetPublishingEntryListResult(EntryListData),

    HandleGetHoldingEntryList(GetListData),
    HandleGetHoldingEntryListResult(EntryListData),

    // -- Meta lists -- //
    HandleGetPublishingMetaList(GetListData),
    HandleGetPublishingMetaListResult(MetaListData),

    HandleGetHoldingMetaList(GetListData),
    HandleGetHoldingMetaListResult(MetaListData),
}
