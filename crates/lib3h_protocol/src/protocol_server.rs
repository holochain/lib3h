use crate::data_types::*;

/// Enum holding all message types in the 'hc-core <- P2P network module' protocol.
/// There are 4 categories of messages:
///  - Command: An order from the local node to the p2p module. Local node expects a reponse. Starts with a verb.
///  - Handle-command: An order from the p2p module to the local node. The p2p module expects a response. Start withs 'Handle' followed by a verb.
///  - Result: A response to a Command. Starts with the name of the Command it responds to and ends with 'Result'.
///  - Notification: Notify that something happened. Not expecting any response. Ends with verb in past form, i.e. '-ed'.
/// Fetch = Request between node and the network (other nodes)
/// Get   = Request within a node between p2p module and core
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "lib3h_server_protocol")]
pub enum Lib3hServerProtocol {
    // -- Generic responses -- //
    /// Success response to a request (any Command with an `request_id` field.)
    SuccessResult(GenericResultData),
    /// Failure response to a request (any Command with an `request_id` field.)
    /// Can also be a response to a mal-formed request.
    FailureResult(GenericResultData),

    // -- Connection -- //
    /// Notification of successful connection to a network
    Connected(ConnectedData),
    /// Notification of disconnection from a network
    Disconnected(DisconnectedData),

    // -- Direct Messaging -- //
    /// the response received from a previous `SendDirectMessage`
    SendDirectMessageResult(DirectMessageData),
    /// Request to handle a direct message another agent has sent us.
    HandleSendDirectMessage(DirectMessageData),

    // -- Entry -- //
    /// Response from requesting dht data from the network
    FetchEntryResult(FetchEntryResultData),
    /// Another node, or the network module itself is requesting data from us
    HandleFetchEntry(FetchEntryData),
    /// Store data on a node's dht arc.
    HandleStoreEntryAspect(StoreEntryAspectData),
    /// Local client does not need to hold that entry anymore.
    /// Local client doesn't 'have to' comply.
    HandleDropEntry(DropEntryData),
    /// Request a node to handle a QueryEntry request
    HandleQueryEntry(QueryEntryData),
    /// Response to a `QueryEntry` request
    QueryEntryResult(QueryEntryResultData),

    // -- Entry lists -- //
    HandleGetAuthoringEntryList(GetListData),
    HandleGetGossipingEntryList(GetListData),

    // -- N3h specific functinonality -- //
    Terminated,
    P2pReady,
}
