use crate::dht::dht_protocol::PeerData;
use lib3h_protocol::{data_types::DirectMessageData, Address}; // HACK

/// Enum holding all message types in the 'network module <-> network module' protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum P2pProtocol {
    // Broadcast(Vec<u8>), // spike
    Gossip(GossipData),
    DirectMessage(DirectMessageData),
    DirectMessageResult(DirectMessageData),
    FetchData,         // TODO
    FetchDataResponse, // TODO
    PeerAddress(String, String),
    // HACK
    JoinSpace(String, PeerData),
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
