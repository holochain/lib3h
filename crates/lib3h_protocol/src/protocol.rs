use crate::{
    data_types::*, protocol_client::Lib3hClientProtocol, protocol_server::Lib3hServerProtocol,
};
/// Enum holding the message types describe the lib3h protocol.
/// There are 4 categories of messages:
///  - ClientToLib3h: A request or event sent from the user/client of lib3h
/// (i.e. the Holochain node) to lib3h.
///  - ClientToLib3hResponse: A response to a ClientToLib3h. The name always matches what it's a response to. The response may have no data associated with it, in which case its use is in the Err
///  - Lib3hToClient: A request or event sent from lib3h its user/client.
///  - Lib3hToClientResponse: A response from the client back to lib3h.
/// Fetch = Request between node and the network (other nodes)
/// Get   = Request within a node between p2p module and core

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientToLib3h {
    // -- Connection -- //
    /// Connect to the specified multiaddr
    Connect(ConnectData),

    // -- Space -- //
    /// Order the engine to be part of the network of the specified space.
    JoinSpace(SpaceData),
    /// Order the engine to leave the network of the specified space.
    LeaveSpace(SpaceData),

    // -- Direct Messaging -- //
    /// Send a message directly to another agent on the network
    SendDirectMessage(DirectMessageData),

    // -- Entry -- //
    /// Request an Entry from the dht network
    FetchEntry(FetchEntryData), // NOTE: MAY BE DEPRECATED
    /// Publish data to the dht (event)
    PublishEntry(ProvidedEntryData),
    /// Tell Engine that Client is holding this entry (event)
    HoldEntry(ProvidedEntryData),
    /// Request some info / data from a Entry
    QueryEntry(QueryEntryData),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientToLib3hResponse {
    /// the response received from a previous `SendDirectMessage`
    SendDirectMessageResult(DirectMessageData),

    /// Response from requesting dht data from the network
    FetchEntryResult(FetchEntryResultData),
    /// Response to a `QueryEntry` request
    QueryEntryResult(QueryEntryResultData),

    /// Notification of successful connection to a network
    ConnectResult(ConnectedData), // response to the ClientToLib3h::Connect() request

    JoinSpaceResult,  // response to the ClientToLib3h::JoinSpace() request, Ok or Err
    LeaveSpaceResult, // response to the ClientToLib3h::LeaveSpace() request, Ok or Err
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Lib3hToClient {
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
pub enum Lib3hToClientResponse {
    /// Our response to a direct message from another agent.
    HandleSendDirectMessageResult(DirectMessageData),
    /// Successful data response for a `HandleFetchEntryData` request
    HandleFetchEntryResult(FetchEntryResultData),
    HandleStoreEntryAspectResult,
    HandleDropEntryResult,
    /// Response to a `HandleQueryEntry` request
    HandleQueryEntryResult(QueryEntryResultData),
    // -- Entry lists -- //
    HandleGetAuthoringEntryListResult(EntryListData),
    HandleGetGossipingEntryListResult(EntryListData),
}

impl From<Lib3hClientProtocol> for ClientToLib3h {
    fn from(c: Lib3hClientProtocol) -> Self {
        match c {
            Lib3hClientProtocol::Connect(connect_data) => ClientToLib3h::Connect(connect_data),
            Lib3hClientProtocol::JoinSpace(space_data) => ClientToLib3h::JoinSpace(space_data),
            Lib3hClientProtocol::LeaveSpace(space_data) => ClientToLib3h::LeaveSpace(space_data),
            Lib3hClientProtocol::SendDirectMessage(direct_message_data) => {
                ClientToLib3h::SendDirectMessage(direct_message_data)
            }
            Lib3hClientProtocol::FetchEntry(fetch_entry_data) => {
                ClientToLib3h::FetchEntry(fetch_entry_data)
            }
            Lib3hClientProtocol::PublishEntry(provided_entry_data) => {
                ClientToLib3h::PublishEntry(provided_entry_data)
            }
            Lib3hClientProtocol::HoldEntry(provided_entry_data) => {
                ClientToLib3h::HoldEntry(provided_entry_data)
            }
            Lib3hClientProtocol::QueryEntry(query_entry_data) => {
                ClientToLib3h::QueryEntry(query_entry_data)
            }
            variant => panic!("{:?} can't convert to ClientToLib3h", variant),
        }
    }
}

impl From<Lib3hClientProtocol> for Lib3hToClientResponse {
    fn from(c: Lib3hClientProtocol) -> Self {
        match c {
            Lib3hClientProtocol::HandleSendDirectMessageResult(direct_message_data) => {
                Lib3hToClientResponse::HandleSendDirectMessageResult(direct_message_data)
            }
            Lib3hClientProtocol::HandleFetchEntryResult(fetch_entry_result_data) => {
                Lib3hToClientResponse::HandleFetchEntryResult(fetch_entry_result_data)
            }
            Lib3hClientProtocol::HandleQueryEntryResult(query_entry_result_data) => {
                Lib3hToClientResponse::HandleQueryEntryResult(query_entry_result_data)
            }
            Lib3hClientProtocol::HandleGetAuthoringEntryListResult(entry_list_data) => {
                Lib3hToClientResponse::HandleGetAuthoringEntryListResult(entry_list_data)
            }
            Lib3hClientProtocol::HandleGetGossipingEntryListResult(entry_list_data) => {
                Lib3hToClientResponse::HandleGetGossipingEntryListResult(entry_list_data)
            }

            variant => panic!("{:?} can't convert to Lib3hToClientResponse", variant),
        }
    }
}

impl From<Lib3hServerProtocol> for Lib3hToClient {
    fn from(c: Lib3hServerProtocol) -> Self {
        match c {
            Lib3hServerProtocol::Connected(connected_data) => {
                Lib3hToClient::Connected(connected_data)
            }
            Lib3hServerProtocol::Disconnected(disconnected_data) => {
                Lib3hToClient::Disconnected(disconnected_data)
            }
            Lib3hServerProtocol::SendDirectMessageResult(direct_message_data) => {
                Lib3hToClient::SendDirectMessageResult(direct_message_data)
            }
            Lib3hServerProtocol::HandleSendDirectMessage(direct_message_data) => {
                Lib3hToClient::HandleSendDirectMessage(direct_message_data)
            }
            Lib3hServerProtocol::HandleFetchEntry(fetch_entry_data) => {
                Lib3hToClient::HandleFetchEntry(fetch_entry_data)
            }
            Lib3hServerProtocol::HandleStoreEntryAspect(store_entry_aspect_data) => {
                Lib3hToClient::HandleStoreEntryAspect(store_entry_aspect_data)
            }
            Lib3hServerProtocol::HandleDropEntry(drop_entry_data) => {
                Lib3hToClient::HandleDropEntry(drop_entry_data)
            }
            Lib3hServerProtocol::HandleQueryEntry(query_entry_data) => {
                Lib3hToClient::HandleQueryEntry(query_entry_data)
            }
            Lib3hServerProtocol::HandleGetAuthoringEntryList(get_list_data) => {
                Lib3hToClient::HandleGetAuthoringEntryList(get_list_data)
            }
            Lib3hServerProtocol::HandleGetGossipingEntryList(get_list_data) => {
                Lib3hToClient::HandleGetGossipingEntryList(get_list_data)
            }
            variant => panic!("{:?} can't convert to Lib3hToClient", variant),
        }
    }
}

impl From<Lib3hServerProtocol> for ClientToLib3hResponse {
    fn from(c: Lib3hServerProtocol) -> Self {
        match c {
            Lib3hServerProtocol::SendDirectMessageResult(direct_message_data) => {
                ClientToLib3hResponse::SendDirectMessageResult(direct_message_data)
            }
            Lib3hServerProtocol::FetchEntryResult(fetch_entry_result_data) => {
                ClientToLib3hResponse::FetchEntryResult(fetch_entry_result_data)
            }

            Lib3hServerProtocol::QueryEntryResult(query_entry_result_data) => {
                ClientToLib3hResponse::QueryEntryResult(query_entry_result_data)
            }
            Lib3hServerProtocol::Connected(connected_data) => {
                ClientToLib3hResponse::ConnectResult(connected_data)
            }
            variant => panic!("{:?} can't convert to ClientToLib3hResponse", variant),
        }
    }
}
///////////////////////////////////////////
impl From<ClientToLib3h> for Lib3hClientProtocol {
    fn from(c: ClientToLib3h) -> Self {
        match c {
            ClientToLib3h::Connect(connect_data) => Lib3hClientProtocol::Connect(connect_data),
            ClientToLib3h::JoinSpace(space_data) => Lib3hClientProtocol::JoinSpace(space_data),
            ClientToLib3h::LeaveSpace(space_data) => Lib3hClientProtocol::LeaveSpace(space_data),
            ClientToLib3h::SendDirectMessage(direct_message_data) => {
                Lib3hClientProtocol::SendDirectMessage(direct_message_data)
            }
            ClientToLib3h::FetchEntry(fetch_entry_data) => {
                Lib3hClientProtocol::FetchEntry(fetch_entry_data)
            }
            ClientToLib3h::PublishEntry(provided_entry_data) => {
                Lib3hClientProtocol::PublishEntry(provided_entry_data)
            }
            ClientToLib3h::HoldEntry(provided_entry_data) => {
                Lib3hClientProtocol::HoldEntry(provided_entry_data)
            }
            ClientToLib3h::QueryEntry(query_entry_data) => {
                Lib3hClientProtocol::QueryEntry(query_entry_data)
            }
        }
    }
}

impl From<Lib3hToClientResponse> for Lib3hClientProtocol {
    fn from(c: Lib3hToClientResponse) -> Self {
        match c {
            Lib3hToClientResponse::HandleSendDirectMessageResult(direct_message_data) => {
                Lib3hClientProtocol::HandleSendDirectMessageResult(direct_message_data)
            }
            Lib3hToClientResponse::HandleFetchEntryResult(fetch_entry_result_data) => {
                Lib3hClientProtocol::HandleFetchEntryResult(fetch_entry_result_data)
            }
            Lib3hToClientResponse::HandleQueryEntryResult(query_entry_result_data) => {
                Lib3hClientProtocol::HandleQueryEntryResult(query_entry_result_data)
            }
            Lib3hToClientResponse::HandleGetAuthoringEntryListResult(entry_list_data) => {
                Lib3hClientProtocol::HandleGetAuthoringEntryListResult(entry_list_data)
            }
            Lib3hToClientResponse::HandleGetGossipingEntryListResult(entry_list_data) => {
                Lib3hClientProtocol::HandleGetGossipingEntryListResult(entry_list_data)
            }
            variant => panic!("{:?} can't convert to Lib3hClientProtocol", variant),
        }
    }
}

impl From<Lib3hToClient> for Lib3hServerProtocol {
    fn from(c: Lib3hToClient) -> Self {
        match c {
            Lib3hToClient::Connected(connected_data) => {
                Lib3hServerProtocol::Connected(connected_data)
            }
            Lib3hToClient::Disconnected(disconnected_data) => {
                Lib3hServerProtocol::Disconnected(disconnected_data)
            }
            Lib3hToClient::SendDirectMessageResult(direct_message_data) => {
                Lib3hServerProtocol::SendDirectMessageResult(direct_message_data)
            },
            Lib3hToClient::HandleSendDirectMessage(direct_message_data) => {
                Lib3hServerProtocol::HandleSendDirectMessage(direct_message_data)
            }
            Lib3hToClient::HandleFetchEntry(fetch_entry_data) => {
                Lib3hServerProtocol::HandleFetchEntry(fetch_entry_data)
            }
            Lib3hToClient::HandleStoreEntryAspect(store_entry_aspect_data) => {
                Lib3hServerProtocol::HandleStoreEntryAspect(store_entry_aspect_data)
            }
            Lib3hToClient::HandleDropEntry(drop_entry_data) => {
                Lib3hServerProtocol::HandleDropEntry(drop_entry_data)
            }
            Lib3hToClient::HandleQueryEntry(query_entry_data) => {
                Lib3hServerProtocol::HandleQueryEntry(query_entry_data)
            }
            Lib3hToClient::HandleGetAuthoringEntryList(get_list_data) => {
                Lib3hServerProtocol::HandleGetAuthoringEntryList(get_list_data)
            }
            Lib3hToClient::HandleGetGossipingEntryList(get_list_data) => {
                Lib3hServerProtocol::HandleGetGossipingEntryList(get_list_data)
            }
        }
    }
}

impl From<ClientToLib3hResponse> for Lib3hServerProtocol {
    fn from(c: ClientToLib3hResponse) -> Self {
        match c {
            ClientToLib3hResponse::SendDirectMessageResult(direct_message_data) => {
                Lib3hServerProtocol::SendDirectMessageResult(direct_message_data)
            }
            ClientToLib3hResponse::FetchEntryResult(fetch_entry_result_data) => {
                Lib3hServerProtocol::FetchEntryResult(fetch_entry_result_data)
            }

            ClientToLib3hResponse::QueryEntryResult(query_entry_result_data) => {
                Lib3hServerProtocol::QueryEntryResult(query_entry_result_data)
            }
            ClientToLib3hResponse::ConnectResult(connected_data) => {
                Lib3hServerProtocol::Connected(connected_data)
            }
            variant => panic!("{:?} can't convert to Lib3hServerProtocol", variant),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    fn connect_data() -> ConnectData {
        ConnectData {
            request_id: "fake_id".to_string(),
            peer_uri: Url::parse("wss://192.168.0.102:58081/").unwrap(),
            network_id: "network_id".into(),
        }
    }

    #[test]
    fn test_translate_protocol() {
        let d = connect_data();
        let s = Lib3hClientProtocol::Connect(d.clone());
        let to_c: ClientToLib3h = s.clone().into();
        assert_eq!(to_c, ClientToLib3h::Connect(d.clone()));

        let to_s: Lib3hClientProtocol = to_c.into();
        assert_eq!(to_s, s);
    }
}
