pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;
pub mod wrapper;

use crate::{
    dht::dht_trait::Dht,
    transport::{protocol::*, transport_trait::Transport, ConnectionId},
};
use detach::prelude::*;
use std::collections::{HashMap, VecDeque};
use url::Url;

/// describes a super construct of a Transport and a Dht allowing
/// Transport access via peer discovery handled by the Dht
pub trait Gateway: Transport + Dht {
    fn identifier(&self) -> &str;
    fn get_connection_id(&self, peer_address: &str) -> Option<String>;
}

/// Gateway to a P2P network.
/// Combines a transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
/// P2pGateway should not `post() & process()` its inner transport but call it synchrounously.
pub struct P2pGateway<D: Dht> {
    child_transport: Detach<TransportParentWrapper>,
    inner_dht: D,
    /// Used for distinguishing gateways
    identifier: String,
    /// Map holding the reversed mapping between connection url and connectionId response
    connection_map: HashMap<Url, ConnectionId>,
    /// Own inbox for TransportCommands which is processed during Transport::process()
    transport_inbox: VecDeque<TransportCommand>,
}
