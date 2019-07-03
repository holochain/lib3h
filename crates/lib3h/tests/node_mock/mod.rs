pub mod chain_store;
pub mod entry_store;
pub mod methods;

use self::chain_store::ChainStore;
use lib3h::engine::RealEngineConfig;
use lib3h_protocol::{
    network_engine::NetworkEngine, protocol_server::Lib3hServerProtocol, Address, Lib3hResult,
};
use std::collections::{HashMap, HashSet};
use url::Url;

static TIMEOUT_MS: usize = 5000;

pub type EngineFactory =
    fn(config: &RealEngineConfig, name: &str) -> Lib3hResult<Box<dyn NetworkEngine>>;

/// Mock of a node handling one agent with multiple Spaces
/// i.e. a conductor mock
pub struct NodeMock {
    /// Temp dir used for persistence
    /// Need to hold the tempdir to keep it alive, otherwise we will get a dir error.
    //_maybe_temp_dir: Option<tempfile::TempDir>,
    /// The Node's networking engine
    engine: Box<dyn NetworkEngine>,
    /// Config used by the engine
    pub config: RealEngineConfig,
    /// The node's simulated agentId
    pub agent_id: Address,
    /// The node's uri
    my_advertise: Url,

    /// Sent messages logs
    request_log: Vec<String>,
    request_count: usize,
    /// Received messages logs
    recv_msg_log: Vec<Lib3hServerProtocol>,

    /// Datastores per Space
    chain_store_list: HashMap<Address, ChainStore>,
    /// List of joined spaces
    joined_space_list: HashSet<Address>,
    /// Space currently in use
    pub current_space: Option<Address>,
}

/// Constructors
impl NodeMock {
    pub fn new_with_config(
        name: &str,
        agent_id_arg: Address,
        config: RealEngineConfig,
        engine_factory: EngineFactory,
        //_maybe_temp_dir: Option<tempfile::TempDir>,
    ) -> Self {
        debug!(
            "new NodeMock '{:?}' with config: {:?}",
            agent_id_arg, config
        );

        let engine = engine_factory(&config, name).expect("Failed to create RealEngine");
        let my_advertise = engine.advertise();
        NodeMock {
            // _maybe_temp_dir,
            engine,
            config,
            agent_id: agent_id_arg.clone(),
            request_log: Vec::new(),
            request_count: 0,
            recv_msg_log: Vec::new(),
            chain_store_list: HashMap::new(),
            joined_space_list: HashSet::new(),
            current_space: None,
            my_advertise,
        }
    }
}
