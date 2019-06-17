mod network_layer;
pub mod p2p_protocol;
pub mod real_engine;
mod space_layer;

use std::collections::{HashMap, VecDeque};

use crate::{
    dht::{
        dht_protocol::{self, *},
        dht_trait::Dht,
        rrdht::RrDht,
    },
    gateway::{gateway_dht, P2pGateway},
    transport::{protocol::*, transport_trait::Transport},
    transport_wss::TransportWss,
};
use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol, Address, AddressRef, DidWork, Lib3hResult,
};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

/// Identifier of a source chain: SpaceAddress+AgentId
pub type ChainId = (Address, Address);

/// Struct holding all config settings for the RealEngine
#[derive(Debug, Clone, PartialEq)]
pub struct RealEngineConfig {
    pub socket_type: String,
    pub bootstrap_nodes: Vec<String>,
    pub work_dir: String,
    pub log_level: char,
}

/// Lib3h's 'real mode' as a NetworkEngine
pub struct RealEngine<'t, T: Transport, D: Dht> {
    /// Config settings
    _config: RealEngineConfig,
    /// FIFO of Lib3hClientProtocol messages received from Core
    inbox: VecDeque<Lib3hClientProtocol>,
    /// Identifier
    name: String,
    network_transport: T,
    /// P2p gateway for the transport layer,
    network_gateway: P2pGateway<'t, T, D>,
    /// Map of P2p gateway per Space+Agent
    space_gateway_map: HashMap<ChainId, P2pGateway<'t, P2pGateway<'t, T, D>, RrDht>>,
}
