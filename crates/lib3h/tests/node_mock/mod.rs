pub mod chain_store;
pub mod entry_store;
pub mod methods;

use self::chain_store::ChainStore;
use lib3h::{engine::RealEngineConfig, error::Lib3hResult};
use lib3h_protocol::{
    error::Lib3hProtocolResult,
    data_types::*,
    network_engine::NetworkEngine, protocol_server::Lib3hServerProtocol, Address,
};
use std::collections::{HashMap, HashSet};
use url::Url;
use crate::utils::processor_harness::*;
use predicates::prelude::*;

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
    pub engine: Box<dyn NetworkEngine>,
    /// Config used by the engine
    pub config: RealEngineConfig,
    /// Factory used to create the engine
    engine_factory: EngineFactory,
    /// The node's simulated agentId
    pub agent_id: Address,
    /// The node's uri
    my_advertise: Url,
    /// This node's handle
    pub name: String,
    /// Keep track of the URIs used when calling `connect()`
    /// in order to do `reconnect()`
    connected_list: HashSet<Url>,

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
            agent_id_arg, config,
        );

        let engine = engine_factory(&config, name).expect("Failed to create RealEngine");
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

pub trait MockApi : lib3h_protocol::network_engine::NetworkEngine {
  
    fn reconnect(&mut self) -> Lib3hProtocolResult<ConnectData>;

    /// Asserts that some event passes an arbitrary predicate
    fn wait_assert(
        &mut self,
        predicate: Box<dyn Predicate<Lib3hServerProtocol>>,
    ) -> Vec<ProcessorResult>;

    /// Asserts some event produced by produce equals actual
    fn wait_eq(&mut self, actual: &Lib3hServerProtocol) -> Vec<ProcessorResult>;

    /// Waits for work to be done
    fn wait_did_work(&mut self, should_abort: bool) -> Vec<ProcessorResult>;

    /// Continues processing the engine until no work is being done.
    fn wait_until_no_work(&mut self) -> Vec<ProcessorResult>;

    fn wait_connect(
        &mut self,
        connect_data: &lib3h_protocol::data_types::ConnectData,
        other: &mut Box<dyn lib3h_protocol::network_engine::NetworkEngine>,
    ) -> Vec<ProcessorResult>;

    fn wait_with_timeout(
        &mut self,
        predicate: Box<dyn Fn(&Lib3hServerProtocol) -> bool>,
        timeout_ms: usize,
    ) -> Option<Lib3hServerProtocol>;
 
    fn wait(
        &mut self,
        predicate: Box<dyn Fn(&Lib3hServerProtocol) -> bool>,
    ) -> Option<Lib3hServerProtocol>;
 
    /// Connect to another peer via its uri
    fn connect_to(&mut self, uri: &Url) -> 
        Lib3hProtocolResult<ConnectData>;

    fn join_space(
        &mut self,
        space_address: &Address,
        can_set_current: bool,
    ) -> Lib3hResult<String>;

    /// Send a DirectMessage on the network.
    /// Returns the generated request_id for this send
    fn send_direct_message(&mut self, to_agent_id: &Address, content: Vec<u8>) -> String;

    /// Send a DirectMessage response on the network.
    fn send_response(
        &mut self,
        request_id: &str,
        to_agent_id: &Address,
        response_content: Vec<u8>,
    );
 
    fn agent_id(&self) -> Address;
    /// Node asks for some entry on the network.
    fn request_entry(&mut self, entry_address: Address) -> QueryEntryData;

    /*fn reply_to_HandleQueryEntry(
        &mut self,
        query: &QueryEntryData,
    ) -> Result<QueryEntryResultData, GenericResultData>;

    ///
    fn reply_to_HandleFetchEntry(
        &mut self,
        fetch: &FetchEntryData,
    ) -> Result<FetchEntryResultData, GenericResultData>;
*/
    ///
    fn author_entry(
        &mut self,
        entry_address: &Address,
        aspect_content_list: Vec<Vec<u8>>,
        can_broadcast: bool,
    ) -> Lib3hResult<EntryData>;

    fn hold_entry(
        &mut self,
        entry_address: &Address,
        aspect_content_list: Vec<Vec<u8>>,
        can_tell_engine: bool,
    ) -> Lib3hResult<EntryData>;
/*
    fn reply_to_first_HandleGetAuthoringEntryList(&mut self);

    /// Reply to a HandleGetGossipingEntryList request
    fn reply_to_HandleGetGossipingEntryList(
        &mut self,
        request: &GetListData,
    ) -> Lib3hResult<()>;

    /// Look for the first HandleGetGossipingEntryList request received from network module and reply
    fn reply_to_first_HandleGetGossipingEntryList(&mut self);
*/
    fn disconnect(&mut self);

    /// Return request id
    fn leave_current_space(&mut self) -> Lib3hResult<String>;
 
    fn set_current_space(&mut self, space_address: &Address);

    /// Return request id
    fn join_current_space(&mut self) -> Lib3hResult<String>;
 
}
