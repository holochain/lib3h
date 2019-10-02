pub mod chain_store;
pub mod entry_store;
pub mod methods;

use self::chain_store::ChainStore;
use lib3h::{
    engine::{ghost_engine_wrapper::WrappedGhostLib3h, EngineConfig},
    error::Lib3hResult,
};
use lib3h_protocol::{protocol_server::Lib3hServerProtocol, uri::Lib3hUri, Address};
use std::collections::{HashMap, HashSet};

static TIMEOUT_MS: usize = 5000;

pub type EngineFactory = fn(config: &EngineConfig, name: &str) -> Lib3hResult<WrappedGhostLib3h>;

/// Mock of a node handling one agent with multiple Spaces
/// i.e. a conductor mock
pub struct NodeMock {
    /// Temp dir used for persistence
    /// Need to hold the tempdir to keep it alive, otherwise we will get a dir error.
    //_maybe_temp_dir: Option<tempfile::TempDir>,
    /// The Node's networking engine
    pub engine: WrappedGhostLib3h,
    /// Config used by the engine
    pub config: EngineConfig,
    /// Factory used to create the engine
    engine_factory: EngineFactory,
    /// The node's simulated agentId
    pub agent_id: Address,
    /// The node's uri
    my_advertise: Lib3hUri,
    /// This node's handle
    pub name: String,
    /// Keep track of the URIs used when calling `connect()`
    /// in order to do `reconnect()`
    connected_list: HashSet<Lib3hUri>,

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
        config: EngineConfig,
        engine_factory: EngineFactory,
        //_maybe_temp_dir: Option<tempfile::TempDir>,
    ) -> Self {
        debug!(
            "new NodeMock '{:?}' with config: {:?}",
            agent_id_arg, config,
        );

        let engine = engine_factory(&config, name).expect("Failed to create Engine");
        let my_advertise = engine.advertise();
        NodeMock {
            // _maybe_temp_dir,
            engine,
            config,
            engine_factory,
            agent_id: agent_id_arg.clone(),
            request_log: Vec::new(),
            request_count: 0,
            recv_msg_log: Vec::new(),
            chain_store_list: HashMap::new(),
            joined_space_list: HashSet::new(),
            current_space: None,
            my_advertise,
            name: name.to_string(),
            connected_list: HashSet::new(),
        }
    }
}

// utility function for tests that rely on nodes joining a space
pub fn test_join_space(node: &mut NodeMock, space_address: &Address) {
    println!("\n {} joins {}\n", node.name(), space_address);
    let req_id = node.join_space(&space_address, true).unwrap();
    let (did_work, srv_msg_list) = node.process().unwrap();
    assert!(did_work);
    assert_eq!(srv_msg_list.len(), 3);
    let msg_1 = &srv_msg_list[0];
    one_let!(Lib3hServerProtocol::SuccessResult(response) = msg_1 {
        assert_eq!(response.request_id, req_id);
    });
}
