use lib3h_protocol::{data_types::DirectMessageData, Lib3hResult, Address};

/// Enum holding all message types in the 'network module <-> network module' protocol.
/// TODO
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum P2pProtocol {
    Broadcast(Vec<u8>),
    Gossip(GossipData),
    DirectMessage(DirectMessageData),
    DirectMessageResult(DirectMessageData),
    SendDirectMessage,
    FetchData,
    FetchDataResponse,
    // FIXME
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct GossipData {
    pub space_address: Address,
    pub to_peer_address: Address,
    pub from_peer_address: Address,
    pub bundle: Vec<u8>,
}