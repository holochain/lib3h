pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;

use crate::{
    track::Tracker,
    dht::dht_trait::Dht,
    transport::{protocol::TransportCommand, transport_trait::Transport, ConnectionId},
};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
enum TrackType {
    /// send messages, log errors, do nothing with success
    TransportSendFireAndForget,
}

/// Gateway to a P2P network.
/// Combines a transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
/// P2pGateway should not `post() & process()` its inner transport but call it synchrounously.
pub struct P2pGateway<T: Transport, D: Dht> {
    inner_transport: Rc<RefCell<T>>,
    inner_dht: D,
    /// Used for distinguishing gateways
    identifier: String,
    /// Tracking request_id's sent to core
    request_track: Tracker<TrackType>,
    /// Map holding the reversed mapping between connection url and connectionId response
    connection_map: HashMap<Url, ConnectionId>,
    /// Own inbox for TransportCommands which is processed during Transport::process()
    transport_inbox: VecDeque<TransportCommand>,
}
