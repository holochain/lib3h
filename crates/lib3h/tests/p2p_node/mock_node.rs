#![allow(non_snake_case)]

use lib3h::{
    engine::{RealEngine, RealEngineConfig},
    transport::{memory_mock::transport_memory::TransportMemory, transport_trait::Transport},
    dht::{dht_trait::Dht, mirror_dht::MirrorDht},
};
use lib3h_protocol::{
    protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol,
    data_types::*,
    Address, AddressRef,
};
use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
};
use lib3h_crypto_api::{FakeCryptoSystem, InsecureBuffer};
use super::{
    chain_store::ChainStore,
    create_config::{create_ipc_config, create_lib3h_config},
};
use crossbeam_channel::{unbounded, Receiver};
use lib3h_protocol::{
    data_types::DirectMessageData, protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol,
};
use multihash::Hash;

static TIMEOUT_MS: usize = 5000;

/// Conductor Mock of one agent with multiple Spaces
pub struct MockNode {
    // Need to hold the tempdir to keep it alive, otherwise we will get a dir error.
    _maybe_temp_dir: Option<tempfile::TempDir>,
    engine: RealEngine<TransportMemory, MirrorDht, InsecureBuffer, FakeCryptoSystem>,
    receiver: Receiver<Lib3hServerProtocol>,
    pub config: RealEngineConfig,

    pub agent_id: Address,

    // my request logging
    request_log: Vec<String>,
    request_count: usize,

    // logging
    recv_msg_log: Vec<Lib3hServerProtocol>,

    // datastores per Space
    chain_store_list: HashMap<Address, ChainStore>,
    joined_space_list: HashSet<Address>,

    pub current_space: Option<Address>,

    is_network_ready: bool,
    pub p2p_binding: String,
}

/// Query logs
impl MockNode {
    /// Return number of JsonProtocol message this node has received
    pub fn count_recv_messages(&self) -> usize {
        self.recv_msg_log.len()
    }

    /// Return the ith JSON message that this node has received and fullfills predicate
    pub fn find_recv_msg(
        &self,
        ith: usize,
        predicate: Box<dyn Fn(&Lib3hServerProtocol) -> bool>,
    ) -> Option<Lib3hServerProtocol> {
        let mut count = 0;
        for msg in self.recv_msg_log.clone() {
            if predicate(&msg) {
                if count == ith {
                    return Some(msg);
                }
                count += 1;
            }
        }
        None
    }
}

/// Space managing
impl MockNode {
    ///
    pub fn join_current_space(&mut self) -> NetResult<()> {
        assert!(self.current_space.is_some());
        let current_space = self.current_space.clone().unwrap();
        self.join_space(&current_space, false)
    }

    pub fn join_space(&mut self, space_address: &Address, can_set_current: bool) -> NetResult<()> {
        if self.joined_space_list.contains(space_address) {
            if can_set_current {
                self.set_current_space(space_address);
            }
            return Ok(());
        }
        let join_space = lib3h_protocol::data_types::SpaceData {
            request_id: "leave_space_req".to_string(),
            space_address: space_address.clone().to_string().into_bytes(),
            agent_id: agent_id.to_string().into_bytes(),
        };
        let protocol_msg = Lib3hClientProtocol::JoinSpace(join_space).into();

        debug!("TestNode.join_space(): {:?}", protocol_msg);
        let res = self.send(protocol_msg);
        if res.is_ok() {
            self.joined_space_list.insert(space_address.clone());
            if !self.chain_store_list.contains_key(space_address) {
                self.chain_store_list
                    .insert(space_address.clone(), ChainStore::new(space_address));
            }
            if can_set_current {
                self.set_current_space(space_address);
            }
        }
        res
    }

    pub fn leave_current_space(&mut self) -> NetResult<()> {
        assert!(self.current_space.is_some());
        let current_space = self.current_space.clone().unwrap();
        let res = self.leave_space(&current_space);
        if res.is_ok() {
            self.current_space = None;
        }
        res
    }

    pub fn leave_space(&mut self, space_address: &Address) -> NetResult<()> {
        if !self.joined_space_list.contains(space_address) {
            return Ok(());
        }
        let agent_id = self.agent_id.clone();
        let leave_space_msg = lib3h_protocol::data_types::SpaceData {
            request_id: "leave_space_req".to_string(),
            space_address: space_address.clone().to_string().into_bytes(),
            agent_id: agent_id.to_string().into_bytes(),
        };
        let protocol_msg = Lib3hClientProtocol::LeaveSpace(leave_space_msg).into();
        let res = self.send(protocol_msg);
        if res.is_ok() {
            self.joined_space_list.remove(space_address);
        }
        res
    }

    ///
    pub fn has_joined(&self, space_address: &Address) -> bool {
        self.joined_space_list.contains(space_address)
    }

    ///
    pub fn set_current_space(&mut self, space_address: &Address) {
        if self.chain_store_list.contains_key(space_address) {
            self.current_space = Some(space_address.clone());
        };
    }
}

///
impl MockNode {
    /// Convert an aspect_content_list into an EntryData
    fn into_EntryData(entry_address: &Address, aspect_content_list: Vec<Vec<u8>>) -> EntryData {
        let mut aspect_list = Vec::new();
        for aspect_content in aspect_content_list {
            let hash = HashString::encode_from_bytes(aspect_content.as_slice(), Hash::SHA2256);
            aspect_list.push(EntryAspectData {
                aspect_address: hash,
                type_hint: "TestNode".to_string(),
                aspect: aspect_content,
                publish_ts: 42,
            });
        }
        EntryData {
            entry_address: entry_address.clone(),
            aspect_list,
        }
    }

    ///
    pub fn author_entry(
        &mut self,
        entry_address: &Address,
        aspect_content_list: Vec<Vec<u8>>,
        can_broadcast: bool,
    ) -> NetResult<()> {
        assert!(self.current_space.is_some());
        let current_space = self.current_space.clone().unwrap();
        let entry = MockNode::into_EntryData(entry_address, aspect_content_list);

        // bookkeep
        {
            let chain_store = self
                .chain_store_list
                .get_mut(&current_space)
                .expect("No chain_store for this Space");
            let res = chain_store.author_entry(&entry);
            // Entry is known, try authoring each aspect instead
            if res.is_err() {
                let mut success = false;
                for aspect in &entry.aspect_list {
                    let aspect_res = chain_store.author_aspect(&entry.entry_address, aspect);
                    if aspect_res.is_ok() {
                        success = true;
                    }
                }
                if !success {
                    return Err(format_err!("Authoring of all aspects failed."));
                }
            }
        }
        if can_broadcast {
            let msg_data = ProvidedEntryData {
                space_address: current_space,
                provider_agent_id: self.agent_id.clone(),
                entry: entry.clone(),
            };
            return self.send(JsonProtocol::PublishEntry(msg_data).into());
        }
        // Done
        Ok(())
    }

    pub fn hold_entry(
        &mut self,
        entry_address: &Address,
        aspect_content_list: Vec<Vec<u8>>,
    ) -> NetResult<()> {
        assert!(self.current_space.is_some());
        let current_space = self.current_space.clone().unwrap();
        let entry = MockNode::into_EntryData(entry_address, aspect_content_list);
        let chain_store = self
            .chain_store_list
            .get_mut(&current_space)
            .expect("No chain_store for this Space");
        let res = chain_store.hold_entry(&entry);
        // Entry is known, try authoring each aspect instead
        if res.is_err() {
            let mut success = false;
            for aspect in entry.aspect_list {
                let aspect_res = chain_store.hold_aspect(&entry.entry_address, &aspect);
                if aspect_res.is_ok() {
                    success = true;
                }
            }
            if !success {
                return Err(format_err!("Storing of all aspects failed."));
            }
        }
        // Done
        Ok(())
    }
}

/// Query & Fetch
impl MockNode {
    /// generate a new request_id
    fn generate_request_id(&mut self) -> String {
        self.request_count += 1;
        let request_id = format!("req_{}_{}", self.agent_id, self.request_count);
        self.request_log.push(request_id.clone());
        request_id
    }

    /// Node asks for some entry on the network.
    pub fn request_entry(&mut self, entry_address: Address) -> QueryEntryData {
        assert!(self.current_space.is_some());
        let current_space = self.current_space.clone().unwrap();
        let query_data = QueryEntryData {
            space_address: current_space,
            entry_address,
            request_id: self.generate_request_id(),
            requester_agent_id: self.agent_id.clone(),
            query: vec![], // empty means give me the EntryData,
        };
        self.send(JsonProtocol::QueryEntry(query_data.clone()).into())
            .expect("Sending Query failed");
        query_data
    }

    ///
    pub fn reply_to_HandleQueryEntry(
        &mut self,
        query: &QueryEntryData,
    ) -> Result<QueryEntryResultData, GenericResultData> {
        // Must be empty query
        if !query.query.is_empty() {
            let msg_data = GenericResultData {
                space_address: query.space_address.clone(),
                request_id: query.request_id.clone(),
                to_agent_id: query.requester_agent_id.clone(),
                result_info: "Unknown query request".as_bytes().to_vec(),
            };
            self.send(JsonProtocol::FailureResult(msg_data.clone()).into())
                .expect("Sending FailureResult failed");
            return Err(msg_data);
        }
        // Convert query to fetch
        let fetch = FetchEntryData {
            space_address: query.space_address.clone(),
            request_id: query.request_id.clone(),
            provider_agent_id: query.requester_agent_id.clone(),
            entry_address: query.entry_address.clone(),
            aspect_address_list: None,
        };
        // HandleFetchEntry
        let fetch_res = self.reply_to_HandleFetchEntry_inner(&fetch);
        if let Err(res) = fetch_res {
            self.send(JsonProtocol::FailureResult(res.clone()).into())
                .expect("Sending FailureResult failed");
            return Err(res);
        }
        // Convert query to fetch
        let query_res = QueryEntryResultData {
            space_address: query.space_address.clone(),
            entry_address: query.entry_address.clone(),
            request_id: query.request_id.clone(),
            requester_agent_id: query.requester_agent_id.clone(),
            responder_agent_id: self.agent_id.clone(),
            query_result: bincode::serialize(&fetch_res.unwrap().entry).unwrap(),
        };
        self.send(JsonProtocol::HandleQueryEntryResult(query_res.clone()).into())
            .expect("Sending FailureResult failed");
        return Ok(query_res);
    }

    ///
    pub fn reply_to_HandleFetchEntry(
        &mut self,
        fetch: &FetchEntryData,
    ) -> Result<FetchEntryResultData, GenericResultData> {
        let fetch_res = self.reply_to_HandleFetchEntry_inner(fetch);
        let json_msg = match fetch_res.clone() {
            Err(res) => JsonProtocol::FailureResult(res),
            Ok(fetch) => JsonProtocol::HandleFetchEntryResult(fetch),
        };
        self.send(json_msg.into()).expect("Sending failed");
        fetch_res
    }

    /// Node asks for some entry on the network.
    fn reply_to_HandleFetchEntry_inner(
        &mut self,
        fetch: &FetchEntryData,
    ) -> Result<FetchEntryResultData, GenericResultData> {
        // Must be tracking Space
        if !self.has_joined(&fetch.space_address) {
            let msg_data = GenericResultData {
                space_address: fetch.space_address.clone(),
                request_id: fetch.request_id.clone(),
                to_agent_id: fetch.provider_agent_id.clone(),
                result_info: "Space is not tracked".as_bytes().to_vec(),
            };
            return Err(msg_data);
        }
        // Get Entry
        let maybe_store = self.chain_store_list.get(&fetch.space_address);
        let maybe_entry = match maybe_store {
            None => None,
            Some(chain_store) => chain_store.get_entry(&fetch.entry_address),
        };
        // No entry, send failure
        if maybe_entry.is_none() {
            let msg_data = GenericResultData {
                space_address: fetch.space_address.clone(),
                request_id: fetch.request_id.clone(),
                to_agent_id: fetch.provider_agent_id.clone(),
                result_info: "No entry found".as_bytes().to_vec(),
            };
            return Err(msg_data);
        }
        // Send EntryData as binary
        let fetch_result_data = FetchEntryResultData {
            space_address: fetch.space_address.clone(),
            provider_agent_id: fetch.provider_agent_id.clone(),
            request_id: fetch.request_id.clone(),
            entry: maybe_entry.unwrap(),
        };
        Ok(fetch_result_data)
    }
}
impl MockNode {
    /// Node sends Message on the network.
    pub fn send_direct_message(&mut self, to_agent_id: &Address, content: Vec<u8>) -> String {
        debug!("current_space: {:?}", self.current_space);
        assert!(self.current_space.is_some());
        let space_address = self.current_space.clone().unwrap();
        let request_id = self.generate_request_id();
        let from_agent_id = self.agent_id.to_string();

        let msg_data = DirectMessageData {
            space_address: space_address.to_string().into_bytes(),
            request_id: request_id.clone(),
            to_agent_id: to_agent_id.to_string().into_bytes(),
            from_agent_id: from_agent_id.to_string().into_bytes(),
            content,
        };
        let p = Lib3hClientProtocol::SendDirectMessage(msg_data.clone()).into();
        self.send(p).expect("Sending SendMessage failed");
        request_id
    }

    /// Node sends Message on the network.
    pub fn send_reponse_json(&mut self, msg: MessageData, response_content: Vec<u8>) {
        assert!(self.current_space.is_some());
        let current_space = self.current_space.clone().unwrap();
        assert_eq!(msg.space_address, current_space.clone());
        assert_eq!(msg.to_agent_id, self.agent_id);
        let response = MessageData {
            space_address: current_space.clone(),
            request_id: msg.request_id,
            to_agent_id: msg.from_agent_id.clone(),
            from_agent_id: self.agent_id.clone(),
            content: response_content,
        };
        self.send(JsonProtocol::HandleSendMessageResult(response.clone()).into())
            .expect("Sending HandleSendMessageResult failed");
    }

    /// Node sends Message on the network.
    pub fn send_reponse_lib3h(
        &mut self,
        msg: DirectMessageData,
        response_content: serde_json::Value,
    ) {
        assert!(self.current_space.is_some());
        let current_space = self.current_space.clone().unwrap();
        assert_eq!(
            msg.space_address,
            current_space.clone().to_string().into_bytes()
        );
        assert_eq!(
            msg.to_agent_id,
            self.agent_id.clone().to_string().into_bytes()
        );
        let response = DirectMessageData {
            space_address: current_space.clone().to_string().into_bytes(),
            request_id: msg.request_id,
            to_agent_id: msg.from_agent_id.clone(),
            from_agent_id: self.agent_id.to_string().into_bytes(),
            content: response_content.to_string().into_bytes(),
        };
        self.send(Lib3hClientProtocol::HandleSendDirectMessageResult(response.clone()).into())
            .expect("Sending HandleSendMessageResult failed");
    }
}

/// Reply LISTS
impl MockNode {
    /// Reply to a HandleGetAuthoringEntryList request
    pub fn reply_to_HandleGetAuthoringEntryList(&mut self, request: &GetListData) -> NetResult<()> {
        assert!(self.current_space.is_some());
        let current_space = self.current_space.clone().unwrap();
        assert_eq!(request.space_address, current_space);
        // Create msg data
        let msg;
        {
            let authored_entry_store = self
                .chain_store_list
                .get_mut(&current_space)
                .expect("No chain_store for this Space")
                .get_authored_store();
            let mut entry_address_list = HashMap::new();
            for (entry_address, entry_map) in authored_entry_store {
                let aspect_map = entry_map
                    .iter()
                    .map(|(a_address, _)| a_address.clone())
                    .collect();
                entry_address_list.insert(entry_address, aspect_map);
            }
            msg = EntryListData {
                request_id: request.request_id.clone(),
                space_address: request.space_address.clone(),
                address_map: entry_address_list,
                provider_agent_id: self.agent_id.clone(),
            };
        }
        self.send(JsonProtocol::HandleGetAuthoringEntryListResult(msg).into())
    }
    /// Look for the first HandleGetAuthoringEntryList request received from network module and reply
    pub fn reply_to_first_HandleGetAuthoringEntryList(&mut self) {
        let request = self
            .find_recv_msg(
                0,
                Box::new(one_is!(JsonProtocol::HandleGetAuthoringEntryList(_))),
            )
            .expect("Did not receive any HandleGetAuthoringEntryList request");
        let get_entry_list_data = unwrap_to!(request => JsonProtocol::HandleGetAuthoringEntryList);
        self.reply_to_HandleGetAuthoringEntryList(&get_entry_list_data)
            .expect("Reply to HandleGetAuthoringEntryList failed.");
    }

    /// Reply to a HandleGetHoldingEntryList request
    pub fn reply_to_HandleGetHoldingEntryList(&mut self, request: &GetListData) -> NetResult<()> {
        assert!(self.current_space.is_some());
        let current_space = self.current_space.clone().unwrap();
        assert_eq!(request.space_address, current_space);
        let msg;
        {
            let stored_entry_store = self
                .chain_store_list
                .get_mut(&current_space)
                .expect("No chain_store for this Space")
                .get_stored_store();
            let mut entry_address_list = HashMap::new();
            for (entry_address, entry_map) in stored_entry_store {
                let aspect_map = entry_map
                    .iter()
                    .map(|(a_address, _)| a_address.clone())
                    .collect();
                entry_address_list.insert(entry_address, aspect_map);
            }
            msg = EntryListData {
                request_id: request.request_id.clone(),
                space_address: request.space_address.clone(),
                address_map: entry_address_list,
                provider_agent_id: self.agent_id.clone(),
            };
        }
        self.send(JsonProtocol::HandleGetGossipingEntryListResult(msg).into())
    }
    /// Look for the first HandleGetHoldingEntryList request received from network module and reply
    pub fn reply_to_first_HandleGetHoldingEntryList(&mut self) {
        let request = self
            .find_recv_msg(
                0,
                Box::new(one_is!(JsonProtocol::HandleGetGossipingEntryList(_))),
            )
            .expect("Did not receive a HandleGetHoldingEntryList request");
        // extract request data
        let get_list_data = unwrap_to!(request => JsonProtocol::HandleGetGossipingEntryList);
        // reply
        self.reply_to_HandleGetHoldingEntryList(&get_list_data)
            .expect("Reply to HandleGetHoldingEntryList failed.");
    }
}

impl MockNode {
    /// Private constructor
    #[cfg_attr(tarpaulin, skip)]
    pub fn new_with_config(
        name: &str,
        agent_id_arg: Address,
        config: RealEngineConfig,
        _maybe_temp_dir: Option<tempfile::TempDir>,
    ) -> Self {
        log_dd!(
            "p2pnode",
            "new TestNode '{}' with config: {:?}",
            agent_id_arg,
            config
        );

        // use a channel for messaging between p2p connection and main thread
        let (sender, receiver) = unbounded::<Lib3hServerProtocol>();
        // create a new P2pNetwork instance with the handler that will send the received Protocol to a channel
        let agent_id = agent_id_arg.clone();
        let engine = RealEngine::new_mock(
                        config, name, MirrorDht::new_with_config,
        )
        .expect("Failed to create RealEngine");

        MockNode {
            _maybe_temp_dir,
            engine,
            receiver,
            config: config.clone(),
            agent_id,
            request_log: Vec::new(),
            request_count: 0,
            recv_msg_log: Vec::new(),
            chain_store_list: HashMap::new(),
            joined_space_list: HashSet::new(),
            current_space: None,
            is_network_ready: false,
            p2p_binding: String::new(),
        }
    }

    #[cfg_attr(tarpaulin, skip)]
    pub fn is_network_ready(&self) -> bool {
        self.is_network_ready
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

    /// Constructor for an IPC node that uses an existing n3h process and a temp folder
    #[cfg_attr(tarpaulin, skip)]
    pub fn new_with_lib3h(
        agent_id: Address,
        maybe_config_filepath: Option<&str>,
        maybe_end_user_config_filepath: Option<String>,
        bootstrap_nodes: Vec<String>,
        maybe_dir_path: Option<String>,
    ) -> Self {
        let (p2p_config, _maybe_temp_dir) = create_lib3h_config(
            maybe_config_filepath,
            maybe_end_user_config_filepath,
            bootstrap_nodes,
            maybe_dir_path,
        );
        return MockNode::new_with_config(agent_id, &p2p_config, _maybe_temp_dir);
    }

    /// Constructor for an IPC node that spawns and uses a n3h process and a temp folder
    #[cfg_attr(tarpaulin, skip)]
    pub fn new_with_spawn_ipc_network(
        agent_id: Address,
        maybe_config_filepath: Option<&str>,
        maybe_end_user_config_filepath: Option<String>,
        bootstrap_nodes: Vec<String>,
        maybe_dir_path: Option<String>,
    ) -> Self {
        let (p2p_config, _maybe_temp_dir) = create_ipc_config(
            maybe_config_filepath,
            maybe_end_user_config_filepath,
            bootstrap_nodes,
            maybe_dir_path,
        );
        return MockNode::new_with_config(agent_id, &p2p_config, _maybe_temp_dir);
    }

    /// See if there is a message to receive, and log it
    /// return a JsonProtocol if the received message is of that type
    #[cfg_attr(tarpaulin, skip)]
    pub fn process(&mut self) -> NetResult<Lib3hServerProtocol> {
        let data = self.receiver.try_recv()?;
        self.recv_msg_log.push(data.clone());
        self.handle_lib3h(r.clone());
        Ok(r)
    }

    /// recv messages until timeout is reached
    /// returns the number of messages it received during listening period
    /// timeout is reset after a message is received
    #[cfg_attr(tarpaulin, skip)]
    pub fn listen(&mut self, timeout_ms: usize) -> usize {
        let mut count: usize = 0;
        let mut time_ms: usize = 0;
        loop {
            let mut has_recved = false;

            if let Ok(p2p_msg) = self.try_recv() {
                self.logger.t(&format!(
                    "({})::listen() - received: {:?}",
                    self.agent_id, p2p_msg,
                ));
                has_recved = true;
                time_ms = 0;
                count += 1;
            }
            if !has_recved {
                std::thread::sleep(std::time::Duration::from_millis(10));
                time_ms += 10;
                if time_ms > timeout_ms {
                    return count;
                }
            }
        }
    }

    /// wait to receive a HandleFetchEntry request and automatically reply
    /// return true if a HandleFetchEntry has been received
    pub fn wait_HandleFetchEntry_and_reply(&mut self) -> bool {
        let maybe_request = self.wait_json(Box::new(one_is!(JsonProtocol::HandleFetchEntry(_))));
        if maybe_request.is_none() {
            return false;
        }
        let request = maybe_request.unwrap();
        // extract msg data
        let fetch_data = unwrap_to!(request => JsonProtocol::HandleFetchEntry);
        // Respond
        self.reply_to_HandleFetchEntry(&fetch_data)
            .expect("Reply to HandleFetchEntry should work");
        true
    }

    /// wait to receive a HandleFetchEntry request and automatically reply
    /// return true if a HandleFetchEntry has been received
    pub fn wait_HandleQueryEntry_and_reply(&mut self) -> bool {
        let maybe_request = self.wait_json(Box::new(one_is!(JsonProtocol::HandleQueryEntry(_))));
        if maybe_request.is_none() {
            return false;
        }
        let request = maybe_request.unwrap();
        // extract msg data
        let query_data = unwrap_to!(request => JsonProtocol::HandleQueryEntry);
        // Respond
        self.reply_to_HandleQueryEntry(&query_data)
            .expect("Reply to HandleFetchEntry should work");
        true
    }

    /// Wait for receiving a message corresponding to predicate
    /// hard coded timeout
    #[cfg_attr(tarpaulin, skip)]
    pub fn wait_lib3h(
        &mut self,
        predicate: Box<dyn Fn(&Lib3hServerProtocol) -> bool>,
    ) -> Option<Lib3hServerProtocol> {
        self.wait_lib3h_with_timeout(predicate, TIMEOUT_MS)
    }

    /// Wait for receiving a message corresponding to predicate until timeout is reached
    pub fn wait_lib3h_with_timeout(
        &mut self,
        predicate: Box<dyn Fn(&Lib3hServerProtocol) -> bool>,
        timeout_ms: usize,
    ) -> Option<Lib3hServerProtocol> {
        let mut time_ms: usize = 0;
        loop {
            let mut did_something = false;

            if let Ok(p2p_msg) = self.try_recv() {
                if let Protocol::Lib3hServer(lib3h_msg) = p2p_msg {
                    self.logger.i(&format!(
                        "({})::wait_lib3h() - received: {:?}",
                        self.agent_id, lib3h_msg
                    ));
                    did_something = true;
                    if predicate(&lib3h_msg) {
                        self.logger
                            .i(&format!("({})::wait_lib3h() - match", self.agent_id));
                        return Some(lib3h_msg);
                    } else {
                        self.logger
                            .i(&format!("({})::wait_lib3h() - NO match", self.agent_id));
                    }
                }
            }

            if !did_something {
                std::thread::sleep(std::time::Duration::from_millis(10));
                time_ms += 10;
                if time_ms > timeout_ms {
                    self.logger
                        .i(&format!("({})::wait_lib3h() has TIMEOUT", self.agent_id));
                    return None;
                }
            }
        }
    }

    // Stop node
    #[cfg_attr(tarpaulin, skip)]
    pub fn stop(self) {
        self.engine
            .stop()
            .expect("Failed to stop p2p connection properly");
    }

    /// Getter of the endpoint of its connection
    #[cfg_attr(tarpaulin, skip)]
    pub fn endpoint(&self) -> String {
        self.engine.endpoint()
    }

    /// handle all types of json message
    #[cfg_attr(tarpaulin, skip)]
    fn handle_lib3h(&mut self, lib3h_msg: Lib3hServerProtocol) {
        match lib3h_msg {
            Lib3hServerProtocol::SuccessResult(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::FailureResult(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::Connected(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::Disconnected(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::SendDirectMessageResult(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::HandleSendDirectMessage(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::FetchEntryResult(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::HandleFetchEntry(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::HandleStoreEntry(_msg) => {
                // FIXME
                if self.has_joined(&msg.space_address) {
                    // Store data in local datastore
                    let mut chain_store = self
                        .chain_store_list
                        .get_mut(&msg.space_address)
                        .expect("No chain_store for this Space");
                    let res = chain_store.hold_aspect(&msg.entry_address, &msg.entry_aspect);
                    self.logger.d(&format!(
                        "({}) auto-store of aspect: {} - {} -> {}",
                        self.agent_id,
                        msg.entry_address,
                        msg.entry_aspect.aspect_address,
                        res.is_ok()
                    ));
                }
            }
            Lib3hServerProtocol::HandleDropEntry(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::HandleGetPublishingEntryList(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::HandleGetHoldingEntryList(_msg) => {
                // FIXME
            }
        }
    }
}

//impl NetSend for TestNode {
//    /// send a Protocol message to the p2p network instance
//    fn send(&mut self, data: Protocol) -> NetResult<()> {
//        self.logger
//            .d(&format!(">> ({}) send: {:?}", self.agent_id, data));
//        self.p2p_connection.send(data)
//    }
//}
