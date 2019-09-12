#[allow(non_snake_case)]
pub mod gateway_actor;
pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;
pub mod protocol;

use crate::{dht::dht_protocol::*, gateway::protocol::*, transport};
use detach::prelude::*;
use lib3h_tracing::Lib3hTrace;

/// Combines a Transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
pub struct P2pGateway {
    /// Used for distinguishing gateways
    identifier: String,

    /// Transport
    child_transport_endpoint:
        Detach<transport::protocol::TransportActorParentContextEndpoint<P2pGateway, Lib3hTrace>>,
    /// DHT
    inner_dht: Detach<ChildDhtWrapperDyn<P2pGateway, Lib3hTrace>>,

    /// self ghost actor
    endpoint_parent: Option<GatewayParentEndpoint>,
    endpoint_self: Detach<GatewaySelfEndpoint<(), Lib3hTrace>>,
    /// cached data from inner dht
    this_peer: PeerData,
}
