mod network_layer;
pub mod p2p_protocol;
pub mod real_engine;
mod space_layer;

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    dht::dht_trait::{Dht, DhtFactory},
    gateway::P2pGateway,
    transport::{transport_trait::Transport, TransportId},
};
use lib3h_protocol::{protocol_client::Lib3hClientProtocol, Address};
use std::{cell::RefCell, rc::Rc};

/// Identifier of a source chain: SpaceAddress+AgentId
pub type ChainId = (Address, Address);

/// Struct holding all config settings for the RealEngine
#[derive(Debug, Clone, PartialEq)]
pub struct RealEngineConfig {
    pub socket_type: String,
    pub bootstrap_nodes: Vec<String>,
    pub work_dir: String,
    pub log_level: char,
    pub bind_url: String,
    pub dht_custom_config: Vec<u8>,
}

/// Lib3h's 'real mode' as a NetworkEngine
pub struct RealEngine<T: Transport, D: Dht> {
    /// Identifier
    name: String,
    /// Config settings
    config: RealEngineConfig,
    /// FIFO of Lib3hClientProtocol messages received from Core
    inbox: VecDeque<Lib3hClientProtocol>,
    /// Factory for building DHT's of type D
    dht_factory: DhtFactory<D>,
    /// P2p gateway for the network layer,
    network_gateway: Rc<RefCell<P2pGateway<T, D>>>,
    /// Store active connections?
    network_connections: HashSet<TransportId>,
    /// Store requests to network layer
    //network_request_map: HashMap<String, TransportCommand>,
    /// Map of P2p gateway per Space+Agent
    space_gateway_map: HashMap<ChainId, P2pGateway<P2pGateway<T, D>, D>>,
}
