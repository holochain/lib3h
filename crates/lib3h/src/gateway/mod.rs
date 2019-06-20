pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;

use crate::{
    dht::dht_trait::Dht,
    transport::{protocol::TransportCommand, transport_trait::Transport, TransportId},
};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

/// Gateway to a P2P network.
/// Combines a transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
/// P2pGateway should not `post() & process()` its inner transport but call it synchrounously.
pub struct P2pGateway<T: Transport, D: Dht> {
    inner_transport: Rc<RefCell<T>>,
    inner_dht: D,
    /// Used for distinguishing gateways
    identifier: String,
    /// Map holding the reversed mapping between connection url and transportId response
    reverse_map: HashMap<String, TransportId>,
    /// Own inbox for TransportCommands which is processed during Transport::process()
    transport_inbox: VecDeque<TransportCommand>,
}
