#[allow(non_snake_case)]
pub mod gateway_actor;
pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;
pub mod protocol;

use crate::{dht::dht_protocol::*, engine::GatewayId, gateway::protocol::*, transport};
use detach::prelude::*;

/// Combines a Transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
pub struct P2pGateway {
    // either network_id or space_address depending on which type of gateway
    identifier: GatewayId,

    /// Transport
    inner_transport: Detach<transport::protocol::TransportActorParentWrapperDyn<Self>>,
    /// DHT
    inner_dht: Detach<ChildDhtWrapperDyn<P2pGateway>>,

    /// self ghost actor
    endpoint_parent: Option<GatewayParentEndpoint>,
    endpoint_self: Detach<GatewaySelfEndpoint<()>>,
    /// cached data from inner dht
    this_peer: PeerData,
}
