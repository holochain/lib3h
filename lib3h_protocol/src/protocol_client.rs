use crate::data_types::*;

/// Enum holding all message types in the 'hc-core -> P2P network module' protocol.
/// There are 4 categories of messages:
///  - Command: An order from the local node to the p2p module. Local node expects a reponse. Starts with a verb.
///  - Handle-command: An order from the p2p module to the local node. The p2p module expects a response. Start withs 'Handle' followed by a verb.
///  - Result: A response to a Command. Starts with the name of the Command it responds to and ends with 'Result'.
///  - Notification: Notify that something happened. Not expecting any response. Ends with verb in past form, i.e. '-ed'.
/// Fetch = Request between node and the network (other nodes)
/// Get   = Request within a node between p2p module and core
#[derive(Debug, Clone, PartialEq)]
pub enum Lib3hClientProtocol {
    // -- Generic responses -- //
    /// Success response to a request (any Command with an `request_id` field.)
    SuccessResult(ResultData),
    /// Failure response to a request (any Command with an `request_id` field.)
    /// Can also be a response to a mal-formed request.
    FailureResult(ResultData),

    // -- Connection -- //
    /// Connect to the specified multiaddr
    Connect(ConnectData),

    // -- DNA tracking -- //
    /// Order the p2p module to be part of the network of the specified DNA.
    TrackDna(TrackDnaData),
    /// Order the p2p module to leave the network of the specified DNA.
    UntrackDna(TrackDnaData),

    // -- Direct Messaging -- //
    /// Send a message directly to another agent on the network
    SendDirectMessage(DirectMessageData),
    /// Our response to a direct message from another agent.
    HandleSendDirectMessageResult(DirectMessageData),

    // -- Entry -- //
    /// Request an Entry (and its meta?) from the dht network
    FetchEntry(FetchEntryData),
    /// Successful data response for a `HandleFetchDhtData` request
    HandleFetchEntryResult(FetchEntryResultData),

    /// Publish data to the dht.
    PublishEntry(ClaimedEntryData),

    // -- Entry lists -- //
    HandleGetPublishingEntryListResult(EntryListData),
    HandleGetHoldingEntryListResult(EntryListData),
}
