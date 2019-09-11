#[allow(non_snake_case)]
pub mod gateway_actor;
pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;
pub mod protocol;

use crate::{dht::dht_protocol::*, gateway::protocol::*, transport};
use detach::prelude::*;
use lib3h_protocol::protocol_server::Lib3hServerProtocol;
use lib3h_tracing::Lib3hTrace;
use url::Url;

/// Combines a Transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
pub struct P2pGateway {
    /// Used for distinguishing gateways
    identifier: String,
    /// Transport
    child_transport_endpoint: Detach<
        transport::protocol::TransportActorParentContextEndpoint<GatewayUserData, Lib3hTrace>,
    >,
    /// DHT
    inner_dht: ChildDhtWrapperDyn<GatewayUserData, Lib3hTrace>,
    // Cache
    this_peer: PeerData,
    // user data for ghost callback
    user_data: GatewayUserData,

    /// self ghost actor
    endpoint_parent: Option<GatewayParentEndpoint>,
    endpoint_self: Detach<GatewaySelfEndpoint<(), Lib3hTrace>>,
}

// user data for ghost callback
pub struct GatewayUserData {
    pub this_peer: PeerData,
    pub maybe_peer: Option<PeerData>,
    pub peer_list: Vec<PeerData>,
    pub lib3h_outbox: Vec<Lib3hServerProtocol>,
    pub binding: Url,
}

impl GatewayUserData {
    pub fn new() -> Self {
        GatewayUserData {
            this_peer: PeerData {
                peer_address: "FIXME".to_string(),
                peer_uri: Url::parse("fixme://host:123").unwrap(),
                timestamp: 0,
            },
            maybe_peer: None,
            peer_list: Vec::new(),
            lib3h_outbox: Vec::new(),
            binding: Url::parse("fixme://host:123").unwrap(),
        }
    }
}
