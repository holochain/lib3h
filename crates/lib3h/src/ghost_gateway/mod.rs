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
    /// Internal DHT
    inner_dht: D,
    /// Hold child transport actor
    child_transport: Detach<TransportParentWrapper>,
    /// Channel to our parent actor
    endpoint_parent: Option<TransportEndpoint>,
    endpoint_self: Option<TransportEndpointWithContext>,
}