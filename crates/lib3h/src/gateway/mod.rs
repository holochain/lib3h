pub mod gateway_transport;
pub mod p2p_gateway;
pub mod wrapper;

use crate::{
    dht::dht_protocol::*,
    transport::{protocol::*, transport_trait::Transport, ConnectionId, TransportWrapper},
};
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::protocol_server::Lib3hServerProtocol;
use std::collections::{HashMap, VecDeque};
use url::Url;
/// describes a super construct of a Transport and a Dht allowing
/// Transport access via peer discovery handled by the Dht
pub trait Gateway<'gateway>: Transport {
    fn identifier(&self) -> &str;
    fn transport_inject_event(&mut self, evt: TransportEvent);
    fn get_connection_id(&mut self, peer_address: &str) -> Option<String>;

    fn process_dht(&mut self) -> GhostResult<()>;
    fn as_dht_mut(&mut self) -> &mut ChildDhtWrapperDyn<'gateway, GatewayUserData>;

    /// temp HACK. Waiting for gateway actor
    fn drain_dht_outbox(&mut self) -> Vec<Lib3hServerProtocol>;

    // TODO - remove this hack
    fn hold_peer(&mut self, peer_data: PeerData);

    // sync actor requests
    fn get_peer_list_sync(&mut self) -> Vec<PeerData>;
    fn get_this_peer_sync(&mut self) -> PeerData;
    fn get_peer_sync(&mut self, peer_address: &str) -> Option<PeerData>;
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
    inner_dht: ChildDhtWrapperDyn<'gateway, GatewayUserData>,
    // Cache
    this_peer: PeerData,
    // user data for ghost callback
    user_data: GatewayUserData,
}

// user data for ghost callback
pub struct GatewayUserData {
    this_peer: PeerData,
    maybe_peer: Option<PeerData>,
    peer_list: Vec<PeerData>,
    pub lib3h_outbox: Vec<Lib3hServerProtocol>,
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
        }
    }
}
