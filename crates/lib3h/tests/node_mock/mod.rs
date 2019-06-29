pub mod chain_store;
pub mod entry_store;
pub mod methods;

use lib3h::{
    engine::RealEngineConfig,
};
use lib3h_protocol::{
    protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol,
    data_types::*,
    Address, AddressRef, Lib3hResult,
    network_engine::NetworkEngine,
};
use std::{
    collections::{HashMap, HashSet},
};
use lib3h_crypto_api::{FakeCryptoSystem, InsecureBuffer};
use self::chain_store::ChainStore;
use crossbeam_channel::{unbounded, Receiver};
// use multihash::Hash;
use url::Url;

static TIMEOUT_MS: usize = 5000;

pub type EngineFactory = fn(config: &RealEngineConfig, name: &str) -> Lib3hResult<Box<dyn NetworkEngine>>;

/// Mock of a node handling one agent with multiple Spaces
/// i.e. a conductor mock
pub struct NodeMock {
    /// Temp dir used for persistence
    /// Need to hold the tempdir to keep it alive, otherwise we will get a dir error.
    //_maybe_temp_dir: Option<tempfile::TempDir>,
    /// TODO: Run engine in a thread and communicate via a channel?
    _receiver: Receiver<Lib3hServerProtocol>,
    /// The Node's networking engine
    engine: Box<dyn NetworkEngine>,
    /// Config used by the engine
    pub config: RealEngineConfig,
    /// The node's simulated agentId
    pub agent_id: Address,

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

    ///
    is_network_ready: bool,

    ///
    my_advertise: Url,
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
            agent_id_arg,
            config
        );

        // Use a channel for messaging between Node and its Engine?
        let (sender, _receiver) = unbounded::<Lib3hServerProtocol>();

        let engine = engine_factory(&config, name).expect("Failed to create RealEngine");
        let my_advertise = engine.advertise();
        NodeMock {
            // _maybe_temp_dir,
            engine,
            _receiver,
            config,
            agent_id: agent_id_arg.clone(),
            request_log: Vec::new(),
            request_count: 0,
            recv_msg_log: Vec::new(),
            chain_store_list: HashMap::new(),
            joined_space_list: HashSet::new(),
            current_space: None,
            is_network_ready: false,
            my_advertise,
        }
    }

//    /// Constructor for an in-memory P2P Network
//    #[cfg_attr(tarpaulin, skip)]
//    pub fn new_with_unique_memory_network(agent_id: Address) -> Self {
//        let config = P2pConfig::new_with_unique_memory_backend();
//        return TestNode::new_with_config(agent_id, &config, None);
//    }

//    /// Constructor for an IPC node that uses an existing n3h process and a temp folder
//    #[cfg_attr(tarpaulin, skip)]
//    pub fn new_with_uri_ipc_network(agent_id: Address, ipc_binding: &str) -> Self {
//        let p2p_config = P2pConfig::default_ipc_uri(Some(ipc_binding));
//        return TestNode::new_with_config(agent_id, &p2p_config, None);
//    }

//    /// Constructor for an IPC node that uses an existing n3h process and a temp folder
//    #[cfg_attr(tarpaulin, skip)]
//    pub fn new_with_lib3h(
//        agent_id: Address,
//        maybe_config_filepath: Option<&str>,
//        maybe_end_user_config_filepath: Option<String>,
//        bootstrap_nodes: Vec<String>,
//        maybe_dir_path: Option<String>,
//    ) -> Self {
//        let (p2p_config, _maybe_temp_dir) = create_lib3h_config(
//            maybe_config_filepath,
//            maybe_end_user_config_filepath,
//            bootstrap_nodes,
//            maybe_dir_path,
//        );
//        return NodeMock::new_with_config(agent_id, &p2p_config, _maybe_temp_dir);
//    }

//    /// Constructor for an IPC node that spawns and uses a n3h process and a temp folder
//    #[cfg_attr(tarpaulin, skip)]
//    pub fn new_with_spawn_ipc_network(
//        agent_id: Address,
//        maybe_config_filepath: Option<&str>,
//        maybe_end_user_config_filepath: Option<String>,
//        bootstrap_nodes: Vec<String>,
//        maybe_dir_path: Option<String>,
//    ) -> Self {
//        let (p2p_config, _maybe_temp_dir) = create_ipc_config(
//            maybe_config_filepath,
//            maybe_end_user_config_filepath,
//            bootstrap_nodes,
//            maybe_dir_path,
//        );
//        return NodeMock::new_with_config(agent_id, &p2p_config, _maybe_temp_dir);
//    }
}