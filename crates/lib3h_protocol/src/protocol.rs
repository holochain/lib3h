use crate::{
    data_types::*, protocol_client::Lib3hClientProtocol, protocol_server::Lib3hServerProtocol,
};
/// Enum holding the message types describe the lib3h protocol.
/// There are 4 categories of messages:
///  - ParentRequest: An a request or event sent from the user/client of lib3h
/// (i.e. the Holochain node) to lib3h.
///  - ParentRequestResponse: A response to a ParentRequest. The name always matches what it's a response to.
///  - ChildRequest: An request or event sent from lib3h it's user/client.
///  - Notification: Notify that something happened. Not expecting any response. Ends with verb in past form, i.e. '-ed'.
/// Fetch = Request between node and the network (other nodes)
/// Get   = Request within a node between p2p module and core

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeToLib3h {
    // -- Connection -- //
    /// Connect to the specified multiaddr
    Connect(ConnectData),

    // -- Space -- //
    /// Order the p2p module to be part of the network of the specified space.
    JoinSpace(SpaceData),
    /// Order the p2p module to leave the network of the specified space.
    LeaveSpace(SpaceData),

    // -- Direct Messaging -- //
    /// Send a message directly to another agent on the network
    SendDirectMessage(DirectMessageData),

    // -- Entry -- //
    /// Request an Entry from the dht network
    FetchEntry(FetchEntryData), // NOTE: MAY BE DEPRECATED
    /// Publish data to the dht.
    PublishEntry(ProvidedEntryData),
    /// Tell network module Core is holding this entry
    HoldEntry(ProvidedEntryData),
    /// Request some info / data from a Entry
    QueryEntry(QueryEntryData),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeToLib3hResponse {
    /// Success response to a request (any Command with an `request_id` field.)
    SuccessResult(GenericResultData),
    /// Failure response to a request (any Command with an `request_id` field.)
    /// Can also be a response to a mal-formed request.
    FailureResult(GenericResultData),

    /// the response received from a previous `SendDirectMessage`
    SendDirectMessageResult(DirectMessageData),
    /// Response from requesting dht data from the network
    FetchEntryResult(FetchEntryResultData),
    /// Response to a `QueryEntry` request
    QueryEntryResult(QueryEntryResultData),

    /// Notification of successful connection to a network
    Connected(ConnectedData), // response to the NodeToLib3h::Connect() request
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Lib3hToNode {
    // -- Connection -- //
    /// Notification of disconnection from a network
    Disconnected(DisconnectedData),

    // -- Direct Messaging -- //
    /// Request to handle a direct message another agent has sent us.
    HandleSendDirectMessage(DirectMessageData),

    // -- Entry -- //
    /// Another node, or the network module itself is requesting data from us
    HandleFetchEntry(FetchEntryData),
    /// Store data on a node's dht arc.
    HandleStoreEntryAspect(StoreEntryAspectData),
    /// Local client does not need to hold that entry anymore.
    /// Local client doesn't 'have to' comply.
    HandleDropEntry(DropEntryData),
    /// Request a node to handle a QueryEntry request
    HandleQueryEntry(QueryEntryData),

    // -- Entry lists -- //
    HandleGetAuthoringEntryList(GetListData),
    HandleGetGossipingEntryList(GetListData),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Lib3hToNodeResponse {
    /// Success response to a request (any Command with an `request_id` field.)
    SuccessResult(GenericResultData),
    /// Failure response to a request (any Command with an `request_id` field.)
    /// Can also be a response to a mal-formed request.
    FailureResult(GenericResultData),

    /// Our response to a direct message from another agent.
    HandleSendDirectMessageResult(DirectMessageData),
    /// Successful data response for a `HandleFetchEntryData` request
    HandleFetchEntryResult(FetchEntryResultData),
    /// Response to a `HandleQueryEntry` request
    HandleQueryEntryResult(QueryEntryResultData),
    // -- Entry lists -- //
    HandleGetAuthoringEntryListResult(EntryListData),
    HandleGetGossipingEntryListResult(EntryListData),
}

impl From<Lib3hClientProtocol> for NodeToLib3h {
    fn from(c: Lib3hClientProtocol) -> Self {
        match c {
            Lib3hClientProtocol::Connect(connect_data) => NodeToLib3h::Connect(connect_data),
            Lib3hClientProtocol::JoinSpace(space_data) => NodeToLib3h::JoinSpace(space_data),
            Lib3hClientProtocol::LeaveSpace(space_data) => NodeToLib3h::LeaveSpace(space_data),
            Lib3hClientProtocol::SendDirectMessage(direct_message_data) => {
                NodeToLib3h::SendDirectMessage(direct_message_data)
            }
            Lib3hClientProtocol::FetchEntry(fetch_entry_data) => {
                NodeToLib3h::FetchEntry(fetch_entry_data)
            }
            Lib3hClientProtocol::PublishEntry(provided_entry_data) => {
                NodeToLib3h::PublishEntry(provided_entry_data)
            }
            Lib3hClientProtocol::HoldEntry(provided_entry_data) => {
                NodeToLib3h::HoldEntry(provided_entry_data)
            }
            Lib3hClientProtocol::QueryEntry(query_entry_data) => {
                NodeToLib3h::QueryEntry(query_entry_data)
            }
            variant => panic!("{:?} can't convert to NodeToLib3h", variant),
        }
    }
}

impl From<Lib3hClientProtocol> for Lib3hToNodeResponse {
    fn from(c: Lib3hClientProtocol) -> Self {
        match c {
            Lib3hClientProtocol::SuccessResult(r) => Lib3hToNodeResponse::SuccessResult(r),
            Lib3hClientProtocol::FailureResult(r) => Lib3hToNodeResponse::FailureResult(r),
            Lib3hClientProtocol::HandleSendDirectMessageResult(direct_message_data) => {
                Lib3hToNodeResponse::HandleSendDirectMessageResult(direct_message_data)
            }
            Lib3hClientProtocol::HandleFetchEntryResult(fetch_entry_result_data) => {
                Lib3hToNodeResponse::HandleFetchEntryResult(fetch_entry_result_data)
            }
            Lib3hClientProtocol::HandleQueryEntryResult(query_entry_result_data) => {
                Lib3hToNodeResponse::HandleQueryEntryResult(query_entry_result_data)
            }
            Lib3hClientProtocol::HandleGetAuthoringEntryListResult(entry_list_data) => {
                Lib3hToNodeResponse::HandleGetAuthoringEntryListResult(entry_list_data)
            }
            Lib3hClientProtocol::HandleGetGossipingEntryListResult(entry_list_data) => {
                Lib3hToNodeResponse::HandleGetGossipingEntryListResult(entry_list_data)
            }

            variant => panic!("{:?} can't convert to Lib3hToNodeResponse", variant),
        }
    }
}

impl From<Lib3hServerProtocol> for Lib3hToNode {
    fn from(c: Lib3hServerProtocol) -> Self {
        match c {
            Lib3hServerProtocol::Disconnected(disconnected_data) => {
                Lib3hToNode::Disconnected(disconnected_data)
            }
            Lib3hServerProtocol::HandleSendDirectMessage(direct_message_data) => {
                Lib3hToNode::HandleSendDirectMessage(direct_message_data)
            }
            Lib3hServerProtocol::HandleFetchEntry(fetch_entry_data) => {
                Lib3hToNode::HandleFetchEntry(fetch_entry_data)
            }
            Lib3hServerProtocol::HandleStoreEntryAspect(store_entry_aspect_data) => {
                Lib3hToNode::HandleStoreEntryAspect(store_entry_aspect_data)
            }
            Lib3hServerProtocol::HandleDropEntry(drop_entry_data) => {
                Lib3hToNode::HandleDropEntry(drop_entry_data)
            }
            Lib3hServerProtocol::HandleQueryEntry(query_entry_data) => {
                Lib3hToNode::HandleQueryEntry(query_entry_data)
            }
            Lib3hServerProtocol::HandleGetAuthoringEntryList(get_list_data) => {
                Lib3hToNode::HandleGetAuthoringEntryList(get_list_data)
            }
            Lib3hServerProtocol::HandleGetGossipingEntryList(get_list_data) => {
                Lib3hToNode::HandleGetGossipingEntryList(get_list_data)
            }
            variant => panic!("{:?} can't convert to Lib3hToNode", variant),
        }
    }
}

impl From<Lib3hServerProtocol> for NodeToLib3hResponse {
    fn from(c: Lib3hServerProtocol) -> Self {
        match c {
            Lib3hServerProtocol::SuccessResult(r) => NodeToLib3hResponse::SuccessResult(r),
            Lib3hServerProtocol::FailureResult(r) => NodeToLib3hResponse::FailureResult(r),
            Lib3hServerProtocol::SendDirectMessageResult(direct_message_data) => {
                NodeToLib3hResponse::SendDirectMessageResult(direct_message_data)
            }
            Lib3hServerProtocol::FetchEntryResult(fetch_entry_result_data) => {
                NodeToLib3hResponse::FetchEntryResult(fetch_entry_result_data)
            }

            Lib3hServerProtocol::QueryEntryResult(query_entry_result_data) => {
                NodeToLib3hResponse::QueryEntryResult(query_entry_result_data)
            }
            Lib3hServerProtocol::Connected(connected_data) => {
                NodeToLib3hResponse::Connected(connected_data)
            }
            variant => panic!("{:?} can't convert to NodeToLib3hResponse", variant),
        }
    }
}
///////////////////////////////////////////
impl From<NodeToLib3h> for Lib3hClientProtocol {
    fn from(c: NodeToLib3h) -> Self {
        match c {
            NodeToLib3h::Connect(connect_data) => Lib3hClientProtocol::Connect(connect_data),
            NodeToLib3h::JoinSpace(space_data) => Lib3hClientProtocol::JoinSpace(space_data),
            NodeToLib3h::LeaveSpace(space_data) => Lib3hClientProtocol::LeaveSpace(space_data),
            NodeToLib3h::SendDirectMessage(direct_message_data) => {
                Lib3hClientProtocol::SendDirectMessage(direct_message_data)
            }
            NodeToLib3h::FetchEntry(fetch_entry_data) => {
                Lib3hClientProtocol::FetchEntry(fetch_entry_data)
            }
            NodeToLib3h::PublishEntry(provided_entry_data) => {
                Lib3hClientProtocol::PublishEntry(provided_entry_data)
            }
            NodeToLib3h::HoldEntry(provided_entry_data) => {
                Lib3hClientProtocol::HoldEntry(provided_entry_data)
            }
            NodeToLib3h::QueryEntry(query_entry_data) => {
                Lib3hClientProtocol::QueryEntry(query_entry_data)
            }
        }
    }
}

impl From<Lib3hToNodeResponse> for Lib3hClientProtocol {
    fn from(c: Lib3hToNodeResponse) -> Self {
        match c {
            Lib3hToNodeResponse::SuccessResult(r) => Lib3hClientProtocol::SuccessResult(r),
            Lib3hToNodeResponse::FailureResult(r) => Lib3hClientProtocol::FailureResult(r),
            Lib3hToNodeResponse::HandleSendDirectMessageResult(direct_message_data) => {
                Lib3hClientProtocol::HandleSendDirectMessageResult(direct_message_data)
            }
            Lib3hToNodeResponse::HandleFetchEntryResult(fetch_entry_result_data) => {
                Lib3hClientProtocol::HandleFetchEntryResult(fetch_entry_result_data)
            }
            Lib3hToNodeResponse::HandleQueryEntryResult(query_entry_result_data) => {
                Lib3hClientProtocol::HandleQueryEntryResult(query_entry_result_data)
            }
            Lib3hToNodeResponse::HandleGetAuthoringEntryListResult(entry_list_data) => {
                Lib3hClientProtocol::HandleGetAuthoringEntryListResult(entry_list_data)
            }
            Lib3hToNodeResponse::HandleGetGossipingEntryListResult(entry_list_data) => {
                Lib3hClientProtocol::HandleGetGossipingEntryListResult(entry_list_data)
            }
        }
    }
}

impl From<Lib3hToNode> for Lib3hServerProtocol {
    fn from(c: Lib3hToNode) -> Self {
        match c {
            Lib3hToNode::Disconnected(disconnected_data) => {
                Lib3hServerProtocol::Disconnected(disconnected_data)
            }
            Lib3hToNode::HandleSendDirectMessage(direct_message_data) => {
                Lib3hServerProtocol::HandleSendDirectMessage(direct_message_data)
            }
            Lib3hToNode::HandleFetchEntry(fetch_entry_data) => {
                Lib3hServerProtocol::HandleFetchEntry(fetch_entry_data)
            }
            Lib3hToNode::HandleStoreEntryAspect(store_entry_aspect_data) => {
                Lib3hServerProtocol::HandleStoreEntryAspect(store_entry_aspect_data)
            }
            Lib3hToNode::HandleDropEntry(drop_entry_data) => {
                Lib3hServerProtocol::HandleDropEntry(drop_entry_data)
            }
            Lib3hToNode::HandleQueryEntry(query_entry_data) => {
                Lib3hServerProtocol::HandleQueryEntry(query_entry_data)
            }
            Lib3hToNode::HandleGetAuthoringEntryList(get_list_data) => {
                Lib3hServerProtocol::HandleGetAuthoringEntryList(get_list_data)
            }
            Lib3hToNode::HandleGetGossipingEntryList(get_list_data) => {
                Lib3hServerProtocol::HandleGetGossipingEntryList(get_list_data)
            }
        }
    }
}

impl From<NodeToLib3hResponse> for Lib3hServerProtocol {
    fn from(c: NodeToLib3hResponse) -> Self {
        match c {
            NodeToLib3hResponse::SuccessResult(r) => Lib3hServerProtocol::SuccessResult(r),
            NodeToLib3hResponse::FailureResult(r) => Lib3hServerProtocol::FailureResult(r),
            NodeToLib3hResponse::SendDirectMessageResult(direct_message_data) => {
                Lib3hServerProtocol::SendDirectMessageResult(direct_message_data)
            }
            NodeToLib3hResponse::FetchEntryResult(fetch_entry_result_data) => {
                Lib3hServerProtocol::FetchEntryResult(fetch_entry_result_data)
            }

            NodeToLib3hResponse::QueryEntryResult(query_entry_result_data) => {
                Lib3hServerProtocol::QueryEntryResult(query_entry_result_data)
            }
            NodeToLib3hResponse::Connected(connected_data) => {
                Lib3hServerProtocol::Connected(connected_data)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generic_result() -> GenericResultData {
        GenericResultData {
            request_id: "fake_id".to_string(),
            space_address: "fake_addr".into(),
            to_agent_id: "fake_addr".into(),
            result_info: b"foo payload".to_vec(),
        }
    }

    #[test]
    fn test_translate_protocol() {
        let gr = generic_result();
        let s = Lib3hServerProtocol::SuccessResult(gr.clone());
        let to_c: NodeToLib3hResponse = s.clone().into();
        assert_eq!(to_c, NodeToLib3hResponse::SuccessResult(gr.clone()));

        let to_s: Lib3hServerProtocol = to_c.into();
        assert_eq!(to_s, s);

        let c = Lib3hServerProtocol::FailureResult(gr.clone());
        let _to_c: NodeToLib3hResponse = c.into();
        //assert_eq!(to_c,NodeToLib3hResponse::FailureResult(gr.clone()));
    }
}
