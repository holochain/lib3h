use lib3h_protocol::{data_types::DirectMessageData, Address, Lib3hResult};

/// Enum holding all message types in the 'network module <-> network module' protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum P2pProtocol {
    Broadcast(Vec<u8>), // spike
    Gossip(GossipData),
    DirectMessage(DirectMessageData),
    DirectMessageResult(DirectMessageData),
    FetchData, // TODO
    FetchDataResponse, // TODO
               // FIXME
}

/// DHT gossip data
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct GossipData {
    pub space_address: Address,
    pub to_peer_address: Address,
    pub from_peer_address: Address,
    pub bundle: Vec<u8>,
}
