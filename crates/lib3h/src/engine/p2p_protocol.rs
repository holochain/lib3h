use crate::dht::dht_protocol::PeerData;
use lib3h_protocol::{data_types::DirectMessageData, Address}; // HACK

pub type SpaceAddress = String;
pub type PeerAddress = String;
pub type GatewayId = String;

/// Enum holding all message types in the 'network module <-> network module' protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum P2pProtocol {
    // Broadcast(Vec<u8>), // spike
    Gossip(GossipData),
    DirectMessage(DirectMessageData),
    DirectMessageResult(DirectMessageData),
    FetchData,         // TODO
    FetchDataResponse, // TODO
    /// Notify another node's our identify in a specific gateway/dht
    PeerAddress(GatewayId, PeerAddress),
    // HACK
    /// Broadcast JoinSpace to all when joining a space
    BroadcastJoinSpace(SpaceAddress, PeerData),
    /// For sending a peer's 'JoinSpace' info to a newly connected peer
    AllJoinedSpaceList(Vec<(SpaceAddress, PeerData)>),
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
