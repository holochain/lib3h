use crate::data_types::*;

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
    /// Success response to a request (any Command with an `request_id` field.)
    SuccessResult(ResultData),
    /// Failure response to a request (any Command with an `request_id` field.)
    /// Can also be a response to a mal-formed request.
    FailureResult(ResultData),

    // -- Connection -- //
    /// Connect to the specified multiaddr
    Connect(ConnectData),
    /// Notification of successful connection to a network
    Connected(ConnectedData),
    /// Notification of disconnection from a network
    Disconnected(DisconnectedData),

    // -- DNA tracking -- //
    /// Order the p2p module to be part of the network of the specified DNA.
    TrackDna(TrackDnaData),
    /// Order the p2p module to leave the network of the specified DNA.
    UntrackDna(TrackDnaData),

    // -- Direct Messaging -- //
    /// Send a message directly to another agent on the network
    SendDirectMessage(DirectMessageData),
    /// the response received from a previous `SendDirectMessage`
    SendDirectMessageResult(DirectMessageData),
    /// Request to handle a direct message another agent has sent us.
    HandleSendDirectMessage(DirectMessageData),
    /// Our response to a direct message from another agent.
    HandleSendDirectMessageResult(DirectMessageData),

    // -- Entry -- //
    /// Request an Entry (and its meta?) from the dht network
    FetchEntry(FetchEntryData),
    /// Response from requesting dht data from the network
    FetchEntryResult(FetchEntryResultData),
    /// Another node, or the network module itself is requesting data from us
    HandleFetchEntry(FetchEntryData),
    /// Successful data response for a `HandleFetchDhtData` request
    HandleFetchEntryResult(FetchEntryResultData),

    /// Publish data to the dht.
    PublishEntry(EntryData),
    /// Store data on a node's dht arc.
    HandleStoreEntry(EntryData),
    /// Local client does not need to hold that entry anymore.
    /// Local client doesn't 'have to' comply.
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