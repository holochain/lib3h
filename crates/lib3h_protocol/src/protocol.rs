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
pub enum ToChild {
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
pub enum ToChildResponse {
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
    Connected(ConnectedData), // response to the ToChild::Connect() request
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToParent {
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
pub enum ToParentResponse {
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

impl From<Lib3hClientProtocol> for ToChild {
    fn from(c: Lib3hClientProtocol) -> Self {
        match c {
            Lib3hClientProtocol::Connect(connect_data) => ToChild::Connect(connect_data),
            Lib3hClientProtocol::JoinSpace(space_data) => ToChild::JoinSpace(space_data),
            Lib3hClientProtocol::LeaveSpace(space_data) => ToChild::LeaveSpace(space_data),
            Lib3hClientProtocol::SendDirectMessage(direct_message_data) => {
                ToChild::SendDirectMessage(direct_message_data)
            }
            Lib3hClientProtocol::FetchEntry(fetch_entry_data) => {
                ToChild::FetchEntry(fetch_entry_data)
            }
            Lib3hClientProtocol::PublishEntry(provided_entry_data) => {
                ToChild::PublishEntry(provided_entry_data)
            }
            Lib3hClientProtocol::HoldEntry(provided_entry_data) => {
                ToChild::HoldEntry(provided_entry_data)
            }
            Lib3hClientProtocol::QueryEntry(query_entry_data) => {
                ToChild::QueryEntry(query_entry_data)
            }
            variant => panic!("{:?} can't convert to ToChild", variant),
        }
    }
}

impl From<Lib3hClientProtocol> for ToParentResponse {
    fn from(c: Lib3hClientProtocol) -> Self {
        match c {
            Lib3hClientProtocol::SuccessResult(r) => ToParentResponse::SuccessResult(r),
            Lib3hClientProtocol::FailureResult(r) => ToParentResponse::FailureResult(r),
            Lib3hClientProtocol::HandleSendDirectMessageResult(direct_message_data) => {
                ToParentResponse::HandleSendDirectMessageResult(direct_message_data)
            }
            Lib3hClientProtocol::HandleFetchEntryResult(fetch_entry_result_data) => {
                ToParentResponse::HandleFetchEntryResult(fetch_entry_result_data)
            }
            Lib3hClientProtocol::HandleQueryEntryResult(query_entry_result_data) => {
                ToParentResponse::HandleQueryEntryResult(query_entry_result_data)
            }
            Lib3hClientProtocol::HandleGetAuthoringEntryListResult(entry_list_data) => {
                ToParentResponse::HandleGetAuthoringEntryListResult(entry_list_data)
            }
            Lib3hClientProtocol::HandleGetGossipingEntryListResult(entry_list_data) => {
                ToParentResponse::HandleGetGossipingEntryListResult(entry_list_data)
            }

            variant => panic!("{:?} can't convert to ToParentResponse", variant),
        }
    }
}

impl From<Lib3hServerProtocol> for ToParent {
    fn from(c: Lib3hServerProtocol) -> Self {
        match c {
            Lib3hServerProtocol::Disconnected(disconnected_data) => {
                ToParent::Disconnected(disconnected_data)
            }
            Lib3hServerProtocol::HandleSendDirectMessage(direct_message_data) => {
                ToParent::HandleSendDirectMessage(direct_message_data)
            }
            Lib3hServerProtocol::HandleFetchEntry(fetch_entry_data) => {
                ToParent::HandleFetchEntry(fetch_entry_data)
            }
            Lib3hServerProtocol::HandleStoreEntryAspect(store_entry_aspect_data) => {
                ToParent::HandleStoreEntryAspect(store_entry_aspect_data)
            }
            Lib3hServerProtocol::HandleDropEntry(drop_entry_data) => {
                ToParent::HandleDropEntry(drop_entry_data)
            }
            Lib3hServerProtocol::HandleQueryEntry(query_entry_data) => {
                ToParent::HandleQueryEntry(query_entry_data)
            }
            Lib3hServerProtocol::HandleGetAuthoringEntryList(get_list_data) => {
                ToParent::HandleGetAuthoringEntryList(get_list_data)
            }
            Lib3hServerProtocol::HandleGetGossipingEntryList(get_list_data) => {
                ToParent::HandleGetGossipingEntryList(get_list_data)
            }
            variant => panic!("{:?} can't convert to ToParent", variant),
        }
    }
}

impl From<Lib3hServerProtocol> for ToChildResponse {
    fn from(c: Lib3hServerProtocol) -> Self {
        match c {
            Lib3hServerProtocol::SuccessResult(r) => ToChildResponse::SuccessResult(r),
            Lib3hServerProtocol::FailureResult(r) => ToChildResponse::FailureResult(r),
            Lib3hServerProtocol::SendDirectMessageResult(direct_message_data) => {
                ToChildResponse::SendDirectMessageResult(direct_message_data)
            }
            Lib3hServerProtocol::FetchEntryResult(fetch_entry_result_data) => {
                ToChildResponse::FetchEntryResult(fetch_entry_result_data)
            }

            Lib3hServerProtocol::QueryEntryResult(query_entry_result_data) => {
                ToChildResponse::QueryEntryResult(query_entry_result_data)
            }
            Lib3hServerProtocol::Connected(connected_data) => {
                ToChildResponse::Connected(connected_data)
            }
            variant => panic!("{:?} can't convert to ToChildResponse", variant),
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
        let c = Lib3hServerProtocol::SuccessResult(gr.clone());
        let to_c: ToChildResponse = c.into();
        assert_eq!(to_c, ToChildResponse::SuccessResult(gr.clone()));
        let c = Lib3hServerProtocol::FailureResult(gr.clone());
        let _to_c: ToChildResponse = c.into();
        //assert_eq!(to_c,ToChild::FailureResult(gr.clone()));
    }
}
