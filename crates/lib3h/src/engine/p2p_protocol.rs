use crate::dht::dht_protocol::PeerData;
use lib3h_protocol::{data_types::DirectMessageData, Address};

pub type SpaceAddress = String;
pub type PeerAddress = String;
pub type GatewayId = String;
pub type PeerTimestamp = u64;

/// Enum holding all message types in the 'network module <-> network module' protocol.
/// TODO #150 - replace this with the p2p-protocol crate
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum P2pProtocol {
    Gossip(GossipData),
    DirectMessage(DirectMessageData),
    DirectMessageResult(DirectMessageData),
    /// Notify another node's our identify in a specific gateway/dht
    PeerAddress(GatewayId, PeerAddress, PeerTimestamp),
    /// Broadcast JoinSpace to all when joining a space
    BroadcastJoinSpace(SpaceAddress, PeerData),
    /// For sending a peer's 'JoinSpace' info to a newly connected peer
    AllJoinedSpaceList(Vec<(SpaceAddress, PeerData)>),
}

/// DHT gossip data
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct GossipData {
    pub space_address: Address,
    pub to_peer_address: Address,
    pub from_peer_address: Address,
    pub bundle: Vec<u8>,
}
