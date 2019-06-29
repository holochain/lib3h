#![allow(non_snake_case)]

use super::{chain_store::ChainStore, NodeMock, TIMEOUT_MS};
use crate::constants::*;
#[allow(unused_imports)]
use crossbeam_channel::{unbounded, Receiver};
use holochain_persistence_api::hash::HashString;
use lib3h_protocol::{
    data_types::*, protocol_client::Lib3hClientProtocol, protocol_server::Lib3hServerProtocol,
    Address, AddressRef, DidWork, Lib3hResult,
};
use multihash::Hash;
use std::collections::HashMap;
use url::Url;

/// Query logs
impl NodeMock {
    /// Return number of Lib3hServerProtocol message this node has received
    pub fn count_recv_messages(&self) -> usize {
        self.recv_msg_log.len()
    }

    /// Return the ith Lib3hServerProtocol message
    /// that this node has received and fullfills predicate
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

    pub fn is_network_ready(&self) -> bool {
        self.is_network_ready
    }

    pub fn advertise(&self) -> Url {
        self.my_advertise.clone()
    }
}

/// Connection & Space managing
impl NodeMock {
    pub fn connect_to(&mut self, uri: &Url) -> Lib3hResult<()> {
        let req_connect = ConnectData {
            request_id: self.generate_request_id(),
            peer_uri: uri.clone(),
            network_id: NETWORK_A_ID.clone(),
        };
        return self
            .engine
            .post(Lib3hClientProtocol::Connect(req_connect.clone()));
    }

    pub fn process(&mut self) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        let (did_work, msgs) = self.engine.process()?;
        self.recv_msg_log.extend_from_slice(msgs.as_slice());
        for msg in msgs.iter() {
            self.handle_lib3h(msg.clone());
        }
        Ok((did_work, msgs))
    }

    ///
    pub fn set_current_space(&mut self, space_address: &Address) {
        if self.chain_store_list.contains_key(space_address) {
            self.current_space = Some(space_address.clone());
        };
    }

    ///
    pub fn join_current_space(&mut self) -> Lib3hResult<()> {
        let current_space = self.current_space.clone().expect("Current space not set");
        self.join_space(&current_space, false)
    }
    ///
    pub fn leave_current_space(&mut self) -> Lib3hResult<()> {
        let current_space = self.current_space.clone().expect("Current space not set");
        let res = self.leave_space(&current_space);
        if res.is_ok() {
            self.current_space = None;
        }
        res
    }

    /// Post a Lib3hClientProtocol::JoinSpace and update internal tracking
    pub fn join_space(
        &mut self,
        space_address: &Address,
        can_set_current: bool,
    ) -> Lib3hResult<()> {
        if self.joined_space_list.contains(space_address) {
            if can_set_current {
                self.set_current_space(space_address);
            }
            return Ok(());
        }
        let join_space = lib3h_protocol::data_types::SpaceData {
            request_id: self.generate_request_id(),
            space_address: space_address.clone(),
            agent_id: self.agent_id.clone(),
        };
        let protocol_msg = Lib3hClientProtocol::JoinSpace(join_space).into();

        debug!("NodeMock.join_space(): {:?}", protocol_msg);
        let res = self.engine.post(protocol_msg);
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

    /// Post a Lib3hClientProtocol::LeaveSpace and update internal tracking
    pub fn leave_space(&mut self, space_address: &Address) -> Lib3hResult<()> {
        if !self.joined_space_list.contains(space_address) {
            return Ok(());
        }
        let agent_id = self.agent_id.clone();
        let leave_space_msg = lib3h_protocol::data_types::SpaceData {
            request_id: self.generate_request_id(),
            space_address: space_address.clone(),
            agent_id,
        };
        let protocol_msg = Lib3hClientProtocol::LeaveSpace(leave_space_msg).into();
        let res = self.engine.post(protocol_msg);
        if res.is_ok() {
            self.joined_space_list.remove(space_address);
        }
        res
    }

    ///
    pub fn has_joined(&self, space_address: &Address) -> bool {
        self.joined_space_list.contains(space_address)
    }
}

///
impl NodeMock {
    /// Convert an aspect_content_list into an EntryData
    fn into_EntryData(entry_address: &Address, aspect_content_list: Vec<Vec<u8>>) -> EntryData {
        let mut aspect_list = Vec::new();
        for aspect_content in aspect_content_list {
            let hash = HashString::encode_from_bytes(aspect_content.as_slice(), Hash::SHA2256);
            aspect_list.push(EntryAspectData {
                aspect_address: hash.to_string().as_bytes().to_vec(),
                type_hint: "NodeMock".to_string(),
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
    ) -> Lib3hResult<()> {
        assert!(self.current_space.is_some());
        let current_space = self.current_space.clone().unwrap();
        let entry = NodeMock::into_EntryData(entry_address, aspect_content_list);

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
            return self
                .engine
                .post(Lib3hClientProtocol::PublishEntry(msg_data).into());
        }
        // Done
        Ok(())
    }

    pub fn hold_entry(
        &mut self,
        entry_address: &Address,
        aspect_content_list: Vec<Vec<u8>>,
    ) -> Lib3hResult<()> {
        assert!(self.current_space.is_some());
        let current_space = self.current_space.clone().unwrap();
        let entry = NodeMock::into_EntryData(entry_address, aspect_content_list);
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
impl NodeMock {
    /// generate a new request_id
    fn generate_request_id(&mut self) -> String {
        self.request_count += 1;
        let request_id = format!("req_{:?}_{}", self.agent_id, self.request_count);
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
        self.engine
            .post(Lib3hClientProtocol::QueryEntry(query_data.clone()).into())
            .expect("Posting Query failed");
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
            self.engine
                .post(Lib3hClientProtocol::FailureResult(msg_data.clone()).into())
                .expect("Posting FailureResult failed");
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
            self.engine
                .post(Lib3hClientProtocol::FailureResult(res.clone()).into())
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
        self.engine
            .post(Lib3hClientProtocol::HandleQueryEntryResult(query_res.clone()).into())
            .expect("Sending FailureResult failed");
        return Ok(query_res);
    }

    ///
    pub fn reply_to_HandleFetchEntry(
        &mut self,
        fetch: &FetchEntryData,
    ) -> Result<FetchEntryResultData, GenericResultData> {
        let fetch_res = self.reply_to_HandleFetchEntry_inner(fetch);
        let msg = match fetch_res.clone() {
            Err(res) => Lib3hClientProtocol::FailureResult(res),
            Ok(fetch) => Lib3hClientProtocol::HandleFetchEntryResult(fetch),
        };
        self.engine.post(msg.into()).expect("Sending failed");
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
impl NodeMock {
    /// Node sends Message on the network.
    /// Returns the generated request_id for this send
    pub fn send_direct_message(&mut self, to_agent_id: &AddressRef, content: Vec<u8>) -> String {
        let current_space = self.current_space.clone().expect("Current Space not set");
        let request_id = self.generate_request_id();
        debug!("current_space: {:?}", self.current_space);
        let msg_data = DirectMessageData {
            space_address: current_space.clone(),
            request_id: request_id.clone(),
            to_agent_id: to_agent_id.to_vec(),
            from_agent_id: self.agent_id.clone(),
            content,
        };
        let p = Lib3hClientProtocol::SendDirectMessage(msg_data.clone()).into();
        self.engine
            .post(p)
            .expect("Posting SendDirectMessage failed");
        request_id
    }

    /// Node sends Message on the network.
    pub fn send_response(
        &mut self,
        request_id: &str,
        to_agent_id: &AddressRef,
        response_content: Vec<u8>,
    ) {
        let current_space = self.current_space.clone().expect("Current Space not set");
        let response = DirectMessageData {
            space_address: current_space.clone(),
            request_id: request_id.to_owned(),
            to_agent_id: to_agent_id.to_vec(),
            from_agent_id: self.agent_id.clone(),
            content: response_content,
        };
        self.engine
            .post(Lib3hClientProtocol::HandleSendDirectMessageResult(response.clone()).into())
            .expect("Posting HandleSendMessageResult failed");
    }
}

/// Reply LISTS
impl NodeMock {
    /// Reply to a HandleGetAuthoringEntryList request
    pub fn reply_to_HandleGetAuthoringEntryList(
        &mut self,
        request: &GetListData,
    ) -> Lib3hResult<()> {
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
        self.engine
            .post(Lib3hClientProtocol::HandleGetAuthoringEntryListResult(msg).into())
    }
    /// Look for the first HandleGetAuthoringEntryList request received from network module and reply
    pub fn reply_to_first_HandleGetAuthoringEntryList(&mut self) {
        let request = self
            .find_recv_msg(
                0,
                Box::new(one_is!(Lib3hServerProtocol::HandleGetAuthoringEntryList(_))),
            )
            .expect("Did not receive any HandleGetAuthoringEntryList request");
        let get_entry_list_data =
            unwrap_to!(request => Lib3hServerProtocol::HandleGetAuthoringEntryList);
        self.reply_to_HandleGetAuthoringEntryList(&get_entry_list_data)
            .expect("Reply to HandleGetAuthoringEntryList failed.");
    }

    /// Reply to a HandleGetHoldingEntryList request
    pub fn reply_to_HandleGetHoldingEntryList(&mut self, request: &GetListData) -> Lib3hResult<()> {
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
        self.engine
            .post(Lib3hClientProtocol::HandleGetGossipingEntryListResult(msg).into())
    }
    /// Look for the first HandleGetHoldingEntryList request received from network module and reply
    pub fn reply_to_first_HandleGetHoldingEntryList(&mut self) {
        let request = self
            .find_recv_msg(
                0,
                Box::new(one_is!(Lib3hServerProtocol::HandleGetGossipingEntryList(_))),
            )
            .expect("Did not receive a HandleGetHoldingEntryList request");
        // extract request data
        let get_list_data = unwrap_to!(request => Lib3hServerProtocol::HandleGetGossipingEntryList);
        // reply
        self.reply_to_HandleGetHoldingEntryList(&get_list_data)
            .expect("Reply to HandleGetHoldingEntryList failed.");
    }

    /// wait to receive a HandleFetchEntry request and automatically reply
    /// return true if a HandleFetchEntry has been received
    pub fn wait_HandleFetchEntry_and_reply(&mut self) -> bool {
        let maybe_request = self.wait(Box::new(one_is!(Lib3hServerProtocol::HandleFetchEntry(_))));
        if maybe_request.is_none() {
            return false;
        }
        let request = maybe_request.unwrap();
        // extract msg data
        let fetch_data = unwrap_to!(request => Lib3hServerProtocol::HandleFetchEntry);
        // Respond
        self.reply_to_HandleFetchEntry(&fetch_data)
            .expect("Reply to HandleFetchEntry should work");
        true
    }

    /// wait to receive a HandleFetchEntry request and automatically reply
    /// return true if a HandleFetchEntry has been received
    pub fn wait_HandleQueryEntry_and_reply(&mut self) -> bool {
        let maybe_request = self.wait(Box::new(one_is!(Lib3hServerProtocol::HandleQueryEntry(_))));
        if maybe_request.is_none() {
            return false;
        }
        let request = maybe_request.unwrap();
        // extract msg data
        let query_data = unwrap_to!(request => Lib3hServerProtocol::HandleQueryEntry);
        // Respond
        self.reply_to_HandleQueryEntry(&query_data)
            .expect("Reply to HandleFetchEntry should work");
        true
    }

    /// Wait for receiving a message corresponding to predicate
    /// hard coded timeout
    pub fn wait(
        &mut self,
        predicate: Box<dyn Fn(&Lib3hServerProtocol) -> bool>,
    ) -> Option<Lib3hServerProtocol> {
        self.wait_with_timeout(predicate, TIMEOUT_MS)
    }

    /// Call process() in a loop until receiving a message corresponding to predicate
    /// or until timeout is reached
    pub fn wait_with_timeout(
        &mut self,
        predicate: Box<dyn Fn(&Lib3hServerProtocol) -> bool>,
        timeout_ms: usize,
    ) -> Option<Lib3hServerProtocol> {
        let mut time_ms: usize = 0;
        loop {
            let (_, msgs) = self.process().expect("Process should work");

            for lib3h_msg in msgs {
                info!("({:?})::wait() - received: {:?}", self.agent_id, lib3h_msg);
                if predicate(&lib3h_msg) {
                    info!("({:?})::wait() - match", self.agent_id);
                    return Some(lib3h_msg);
                } else {
                    info!("({:?})::wait() - NO match", self.agent_id);
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
            time_ms += 10;
            if time_ms > timeout_ms {
                info!("({:?})::wait() has TIMEOUT", self.agent_id);
                return None;
            }
        }
    }

    /// Call process until timeout is reached
    /// returns the number of messages it received during listening period
    /// timeout is reset after a message is received
    pub fn listen(&mut self, timeout_ms: usize) -> usize {
        let mut count: usize = 0;
        let mut time_ms: usize = 0;
        loop {
            let (_, msgs) = self.process().expect("Process should work");

            for lib3h_msg in msgs {
                trace!(
                    "({:?})::listen() - received: {:?}",
                    self.agent_id,
                    lib3h_msg,
                );
                time_ms = 0;
                count += 1;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            time_ms += 10;
            if time_ms > timeout_ms {
                return count;
            }
        }
    }

    /// Start engine
    pub fn run(&mut self) {
        self.engine
            .run()
            .expect("Failed to start the NetworkEngine");
    }

    /// Stop engine
    pub fn stop(&mut self) {
        self.engine
            .stop()
            .expect("Failed to stop the NetworkEngine");
    }

    /// handle all types of Lib3hServerProtocol message
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
            Lib3hServerProtocol::HandleStoreEntryAspect(msg) => {
                // FIXME
                if self.has_joined(&msg.space_address) {
                    // Store data in local datastore
                    let chain_store = self
                        .chain_store_list
                        .get_mut(&msg.space_address)
                        .expect("No chain_store for this Space");
                    let res = chain_store.hold_aspect(&msg.entry_address, &msg.entry_aspect);
                    debug!(
                        "({:?}) auto-store of aspect: {:?} - {:?} -> {}",
                        self.agent_id,
                        msg.entry_address,
                        msg.entry_aspect.aspect_address,
                        res.is_ok()
                    );
                }
            }
            Lib3hServerProtocol::HandleDropEntry(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::HandleQueryEntry(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::QueryEntryResult(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::HandleGetAuthoringEntryList(_msg) => {
                // FIXME
            }
            Lib3hServerProtocol::HandleGetGossipingEntryList(_msg) => {
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
