mod network_layer;
pub mod p2p_protocol;
pub mod real_engine;
mod space_layer;

use std::collections::{HashMap, VecDeque};

use crate::{
    dht::{
        dht_trait::{Dht, DhtFactory},
        rrdht::RrDht,
    },
    gateway::P2pGateway,
    transport::transport_trait::Transport,
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
    pub dht_config: Vec<u8>,
}

/// Lib3h's 'real mode' as a NetworkEngine
pub struct RealEngine<T: Transport, D: Dht> {
    /// Identifier
    name: String,
    /// Config settings
    config: RealEngineConfig,
    /// FIFO of Lib3hClientProtocol messages received from Core
    inbox: VecDeque<Lib3hClientProtocol>,
    ///
    dht_factory: DhtFactory<D>,
    /// P2p gateway for the network layer,
    network_gateway: Rc<RefCell<P2pGateway<T, D>>>,
    /// Map of P2p gateway per Space+Agent
    space_gateway_map: HashMap<ChainId, P2pGateway<P2pGateway<T, D>, D>>,
}
