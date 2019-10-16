use crate::{
    data_types::*,
    error::{ErrorKind, Lib3hProtocolError},
    protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol,
    uri::Lib3hUri,
};

use std::convert::TryFrom;

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
    /// create an explicit connection to a remote peer
    Bootstrap(BootstrapData),

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
    /// Request some info / data from a Entry
    QueryEntry(QueryEntryData),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientToLib3hResponse {
    /// we were able to bootstrap to the remote
    BootstrapSuccess,

    /// the response received from a previous `SendDirectMessage`
    SendDirectMessageResult(DirectMessageData),

    /// Response from requesting dht data from the network
    FetchEntryResult(FetchEntryResultData),
    /// Response to a `QueryEntry` request
    QueryEntryResult(QueryEntryResultData),

    JoinSpaceResult,  // response to the ClientToLib3h::JoinSpace() request, Ok or Err
    LeaveSpaceResult, // response to the ClientToLib3h::LeaveSpace() request, Ok or Err
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Lib3hToClient {
    // -- Connection -- //
    /// Notification of successful connection to a network
    Connected(ConnectedData),
    /// Notification of disconnection from a network
    Unbound(UnboundData),

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

impl TryFrom<Lib3hClientProtocol> for ClientToLib3h {
    type Error = Lib3hProtocolError;
    fn try_from(c: Lib3hClientProtocol) -> Result<Self, Self::Error> {
        match c {
            Lib3hClientProtocol::Connect(connect_data) => {
                Ok(ClientToLib3h::Bootstrap(BootstrapData {
                    space_address: connect_data.network_id.into(),
                    bootstrap_uri: connect_data.peer_location,
                }))
            }
            Lib3hClientProtocol::JoinSpace(space_data) => Ok(ClientToLib3h::JoinSpace(space_data)),
            Lib3hClientProtocol::LeaveSpace(space_data) => {
                Ok(ClientToLib3h::LeaveSpace(space_data))
            }
            Lib3hClientProtocol::SendDirectMessage(direct_message_data) => {
                Ok(ClientToLib3h::SendDirectMessage(direct_message_data))
            }
            Lib3hClientProtocol::FetchEntry(fetch_entry_data) => {
                Ok(ClientToLib3h::FetchEntry(fetch_entry_data))
            }
            Lib3hClientProtocol::PublishEntry(provided_entry_data) => {
                Ok(ClientToLib3h::PublishEntry(provided_entry_data))
            }
            Lib3hClientProtocol::QueryEntry(query_entry_data) => {
                Ok(ClientToLib3h::QueryEntry(query_entry_data))
            }
            variant => Err(Lib3hProtocolError::new(ErrorKind::Other(format!(
                "{:?} can't convert to ClientToLib3h",
                variant
            )))),
        }
    }
}

impl TryFrom<Lib3hClientProtocol> for Lib3hToClientResponse {
    type Error = Lib3hProtocolError;
    fn try_from(c: Lib3hClientProtocol) -> Result<Self, Self::Error> {
        match c {
            Lib3hClientProtocol::HandleSendDirectMessageResult(direct_message_data) => Ok(
                Lib3hToClientResponse::HandleSendDirectMessageResult(direct_message_data),
            ),
            Lib3hClientProtocol::HandleFetchEntryResult(fetch_entry_result_data) => Ok(
                Lib3hToClientResponse::HandleFetchEntryResult(fetch_entry_result_data),
            ),
            Lib3hClientProtocol::HandleQueryEntryResult(query_entry_result_data) => Ok(
                Lib3hToClientResponse::HandleQueryEntryResult(query_entry_result_data),
            ),
            Lib3hClientProtocol::HandleGetAuthoringEntryListResult(entry_list_data) => Ok(
                Lib3hToClientResponse::HandleGetAuthoringEntryListResult(entry_list_data),
            ),
            Lib3hClientProtocol::HandleGetGossipingEntryListResult(entry_list_data) => Ok(
                Lib3hToClientResponse::HandleGetGossipingEntryListResult(entry_list_data),
            ),
            variant => Err(Lib3hProtocolError::new(ErrorKind::Other(format!(
                "{:?} can't convert to Lib3hToClientResponse",
                variant
            )))),
        }
    }
}

impl TryFrom<Lib3hServerProtocol> for Lib3hToClient {
    type Error = Lib3hProtocolError;
    fn try_from(c: Lib3hServerProtocol) -> Result<Self, Self::Error> {
        match c {
            Lib3hServerProtocol::Connected(connected_data) => {
                Ok(Lib3hToClient::Connected(connected_data))
            }
            Lib3hServerProtocol::Disconnected(_disconnected_data) => {
                Ok(Lib3hToClient::Unbound(UnboundData {
                    uri: Lib3hUri::with_undefined(),
                }))
            }
            Lib3hServerProtocol::SendDirectMessageResult(direct_message_data) => {
                Ok(Lib3hToClient::SendDirectMessageResult(direct_message_data))
            }
            Lib3hServerProtocol::HandleSendDirectMessage(direct_message_data) => {
                Ok(Lib3hToClient::HandleSendDirectMessage(direct_message_data))
            }
            Lib3hServerProtocol::HandleFetchEntry(fetch_entry_data) => {
                Ok(Lib3hToClient::HandleFetchEntry(fetch_entry_data))
            }
            Lib3hServerProtocol::HandleStoreEntryAspect(store_entry_aspect_data) => Ok(
                Lib3hToClient::HandleStoreEntryAspect(store_entry_aspect_data),
            ),
            Lib3hServerProtocol::HandleDropEntry(drop_entry_data) => {
                Ok(Lib3hToClient::HandleDropEntry(drop_entry_data))
            }
            Lib3hServerProtocol::HandleQueryEntry(query_entry_data) => {
                Ok(Lib3hToClient::HandleQueryEntry(query_entry_data))
            }
            Lib3hServerProtocol::HandleGetAuthoringEntryList(get_list_data) => {
                Ok(Lib3hToClient::HandleGetAuthoringEntryList(get_list_data))
            }
            Lib3hServerProtocol::HandleGetGossipingEntryList(get_list_data) => {
                Ok(Lib3hToClient::HandleGetGossipingEntryList(get_list_data))
            }
            variant => Err(Lib3hProtocolError::new(ErrorKind::Other(format!(
                "{:?} can't convert to Lib3hToClient",
                variant
            )))),
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
            Lib3hServerProtocol::Connected(_connected_data) => {
                ClientToLib3hResponse::BootstrapSuccess
            }
            variant => panic!("{:?} can't convert to ClientToLib3hResponse", variant),
        }
    }
}
///////////////////////////////////////////
impl From<ClientToLib3h> for Lib3hClientProtocol {
    fn from(c: ClientToLib3h) -> Self {
        match c {
            ClientToLib3h::Bootstrap(bootstrap_data) => Lib3hClientProtocol::Connect(ConnectData {
                request_id: "".to_string(),
                peer_location: bootstrap_data.bootstrap_uri,
                network_id: bootstrap_data.space_address.into(),
            }),
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
            Lib3hToClient::Unbound(_unbound_data) => {
                Lib3hServerProtocol::Disconnected(DisconnectedData {
                    network_id: "".into(),
                })
            }
            Lib3hToClient::SendDirectMessageResult(direct_message_data) => {
                Lib3hServerProtocol::SendDirectMessageResult(direct_message_data)
            }
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
            ClientToLib3hResponse::BootstrapSuccess => {
                Lib3hServerProtocol::Connected(ConnectedData {
                    request_id: String::new(),
                    uri: Lib3hUri::with_undefined(),
                })
            }
            variant => panic!("{:?} can't convert to Lib3hServerProtocol", variant),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;
    use url::Url;

    fn connect_data() -> ConnectData {
        ConnectData {
            request_id: "".to_string(),
            peer_location: Url::parse("wss://192.168.0.102:58081/").unwrap().into(),
            network_id: "network_id".into(),
        }
    }

    #[test]
    fn test_translate_protocol() {
        let d = connect_data();
        let s = Lib3hClientProtocol::Connect(d.clone());
        let to_c: ClientToLib3h = s.clone().try_into().expect("A ClientToLib3h protocol");
        assert_eq!(
            to_c,
            ClientToLib3h::Bootstrap(BootstrapData {
                bootstrap_uri: Url::parse("wss://192.168.0.102:58081/").unwrap().into(),
                space_address: "network_id".to_string().into(),
            })
        );

        let to_s: Lib3hClientProtocol = to_c.into();
        assert_eq!(to_s, s);
    }
}
