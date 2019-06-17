pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;

use crate::{dht::dht_trait::Dht, transport::transport_trait::Transport};
use std::{cell::RefCell, rc::Rc};

/// Gateway to a P2P network.
/// Combines a transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
/// P2pGateway should not `post() & process()` its inner transport but call it synchrounously.
pub struct P2pGateway<T: Transport, D: Dht> {
    inner_transport: Rc<RefCell<T>>,
    inner_dht: D,
    maybe_advertise: Option<String>,
}
