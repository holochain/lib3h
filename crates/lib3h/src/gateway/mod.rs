pub mod gateway_actor;
pub mod gateway_transport;
pub mod gateway_dht;
pub mod p2p_gateway;
pub mod protocol;
// pub mod wrapper;

use crate::{
    dht::dht_protocol::*,
    gateway::protocol::*,
    transport,
};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::protocol_server::Lib3hServerProtocol;
use url::Url;

///// describes a super construct of a Transport and a Dht allowing
///// Transport access via peer discovery handled by the Dht
//pub trait Gateway {
//    fn process_dht(&mut self) -> GhostResult<()>;
//    fn as_dht_mut(&mut self) -> &mut ChildDhtWrapperDyn<GatewayUserData>;
//
//    /// temp HACK. Waiting for gateway actor
//    fn drain_dht_outbox(&mut self) -> Vec<Lib3hServerProtocol>;
//
//    // sync actor requests
//    fn get_peer_list_sync(&mut self) -> Vec<PeerData>;
//    fn get_this_peer_sync(&mut self) -> PeerData;
//    fn get_peer_sync(&mut self, peer_address: &str) -> Option<PeerData>;
//}

/// Gateway to a P2P network.
/// Combines a transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
/// P2pGateway should not `post() & process()` its inner transport but call it synchrounously.
pub struct P2pGateway {
    /// Used for distinguishing gateways
    identifier: String,
    /// Transport
    child_transport_endpoint: transport::protocol::TransportEndpointWithContext<GatewayUserData, GatewayContext>,
    /// DHT
    inner_dht: ChildDhtWrapperDyn<GatewayUserData>,
    // Cache
    this_peer: PeerData,
    // user data for ghost callback
    user_data: GatewayUserData,

    /// self ghost stuff
    endpoint_parent: Option<GatewayParentEndpoint>,
    endpoint_self: Detach<GatewaySelfEndpoint<(), GatewayContext>>,
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
