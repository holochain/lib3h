use crate::dht::dht_protocol::PeerData;
use lib3h_protocol::{
    data_types::{DirectMessageData, Opaque},
    uri::Lib3hUri,
    Address,
};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

pub type SpaceAddress = String;
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
    PeerName(GatewayId, Lib3hUri, PeerTimestamp),
    /// Broadcast JoinSpace to all when joining a space
    BroadcastJoinSpace(SpaceAddress, PeerData),
    /// For sending a peer's 'JoinSpace' info to a newly connected peer
    AllJoinedSpaceList(Vec<(SpaceAddress, PeerData)>),
    /// We would like to transition to using our capnproto p2p protocol
    /// during the transition phase, capnproto messages will be
    /// doubly encoded in this P2pProtocol enum variant,
    /// once all messages have transitioned, we can drop this layer
    CapnProtoMessage(Vec<u8>),
}

/// DHT gossip data
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct GossipData {
    pub space_address: Address,
    pub to_peer_name: Lib3hUri,
    pub from_peer_name: Lib3hUri,
    pub bundle: Opaque,
}

impl P2pProtocol {
    /// rust messagepack decode these bytes into a P2pProtocol instance
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, rmp_serde::decode::Error> {
        let mut de = Deserializer::new(&bytes[..]);
        Deserialize::deserialize(&mut de)
    }

    /// encode this P2pProtocol instance as rust messagepack bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.serialize(&mut Serializer::new(&mut out)).unwrap();
        out
    }

    /// convert this P2pProtocol instance into rust messagepack bytes
    pub fn into_bytes(self) -> Vec<u8> {
        self.to_bytes()
    }
}
