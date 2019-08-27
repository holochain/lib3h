pub mod gateway_actor;
pub mod ghost_gateway;
pub mod gateway_dht;

use lib3h_ghost_actor::{
    prelude::*,
    ghost_channel::*,
};

/// Gateway Actor where:
///   - child: some Transport
///   - parent: RealEngine or Multiplexer
/// Transport protocol used on all ends
pub struct GhostGateway<'gateway, D: Dht> {
    /// Used for distinguishing gateways
    identifier: String,
    /// Map holding the reversed mapping between connection url and connectionId response
    connection_map: HashMap<Url, ConnectionId>,
    /// Internal DHT
    inner_dht: D,
    /// Hold Endpoint to child actor
    // inner_transport: TransportWrapper<'gateway>,
    child_transport: Detach<TransportParentEndpointWithContext>,
    /// Channel to our parent actor
    endpoint_parent: Option<TransportEndpoint>,
    endpoint_self: Option<TransportEndpointWithContext>,
}