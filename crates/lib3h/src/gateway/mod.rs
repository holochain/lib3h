pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;
pub mod wrapper;

use crate::{
    transport::{protocol::*, transport_trait::Transport, ConnectionId, TransportWrapper},
    dht::ghost_protocol::*, dht::dht_protocol::*,
};
use std::collections::{HashMap, VecDeque};
use detach::prelude::*;
use url::Url;

/// describes a super construct of a Transport and a Dht allowing
/// Transport access via peer discovery handled by the Dht
pub trait Gateway: Transport {
    fn identifier(&self) -> &str;
    fn transport_inject_event(&mut self, evt: TransportEvent);
    fn get_connection_id(&self, peer_address: &str) -> Option<String>;

    // sync actor requests
    fn get_peer_list_sync(&mut self) -> Vec<PeerData>;
    fn get_this_peer_sync(&mut self) -> PeerData;
    fn get_peer_sync(&mut self, peer_address: &str) -> Option<PeerData>;

    fn as_dht_mut(&mut self) -> &mut Detach<ChildDhtWrapperDyn>;
}

/// Gateway to a P2P network.
/// Combines a transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
/// P2pGateway should not `post() & process()` its inner transport but call it synchrounously.
pub struct P2pGateway<'gateway> {
    /// Used for distinguishing gateways
    identifier: String,
    /// Map holding the reversed mapping between connection url and connectionId response
    connection_map: HashMap<Url, ConnectionId>,
    /// Own inbox for TransportCommands which is processed during Transport::process()
    inner_transport: TransportWrapper<'gateway>,
    transport_inbox: VecDeque<TransportCommand>,
    transport_inject_events: Vec<TransportEvent>,
    /// DHT
    inner_dht: Detach<ChildDhtWrapperDyn>,
}
