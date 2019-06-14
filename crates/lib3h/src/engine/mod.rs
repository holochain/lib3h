pub mod p2p_protocol;
pub mod real_engine;
mod space_layer;
mod network_layer;

use lib3h_protocol::Address;

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
