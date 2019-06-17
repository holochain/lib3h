pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;

use crate::{
    dht::dht_trait::Dht, transport::transport_trait::Transport, transport_wss::TransportWss,
};

/// Gateway to a P2P network.
/// Enables Connections to many other nodes.
/// Tracks distributed data for that P2P network.
/// P2pGateway should not `post() & process()` its inner transport but call it synchrounously.
/// Composite pattern for Transport and Dht
pub struct P2pGateway<'t, T: Transport, D: Dht> {
    inner_transport: &'t T,
    inner_dht: D,
    maybe_advertise: Option<String>,
}
