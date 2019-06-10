#![allow(non_snake_case)]

//#[cfg(test)]
use crate::transport::memory_mock::transport_memory::TransportMemory;
use std::collections::{HashMap, VecDeque};

use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol, Address, DidWork, Lib3hResult,
};

use crate::{
    dht::{
        dht_event::{DhtEvent, PeerHoldRequestData},
        dht_trait::Dht,
        rrdht::RrDht,
    },
    p2p::{p2p_gateway::P2pGateway, p2p_protocol::P2pProtocol},
    transport::transport_trait::Transport,
    transport_space::TransportSpace,
    transport_wss::TransportWss,
};

/// Identifier of a source chain: SpaceAddress+AgentId
pub type PlayerId = (Address, Address);

/// Struct holding all config settings for the RealEngine
#[derive(Debug, Clone, PartialEq)]
pub struct RealEngineConfig {
    pub socket_type: String,
    pub bootstrap_nodes: Vec<String>,
    pub work_dir: String,
    pub log_level: char,
}

/// Lib3h's 'real mode' as a NetworkEngine
pub struct RealEngine<T: Transport> {
    /// Config settings
    _config: RealEngineConfig,
    /// FIFO of Lib3hClientProtocol messages received from Core
    inbox: VecDeque<Lib3hClientProtocol>,
    /// Identifier
    name: String,
    /// P2p gateway for the transport layer,
    transport_gateway: P2pGateway<T, RrDht>,
    /// Map of P2p gateway per Space+Agent
    space_gateway_map: HashMap<PlayerId, P2pGateway<TransportSpace, RrDht>>,
}

impl RealEngine<TransportWss<std::net::TcpStream>> {
    /// Constructor
    pub fn new(config: RealEngineConfig, name: &str) -> Lib3hResult<Self> {
        Ok(RealEngine {
            _config: config,
            inbox: VecDeque::new(),
            name: name.to_string(),
            transport_gateway: P2pGateway::new_with_wss(),
            space_gateway_map: HashMap::new(),
        })
    }
}

/// Constructor
//#[cfg(test)]
impl RealEngine<TransportMemory> {
    pub fn new_mock(config: RealEngineConfig, name: &str) -> Lib3hResult<Self> {
        Ok(RealEngine {
            _config: config,
            inbox: VecDeque::new(),
            name: name.to_string(),
            transport_gateway: P2pGateway::new_with_memory(name),
            space_gateway_map: HashMap::new(),
        })
    }
}

impl<T: Transport> NetworkEngine for RealEngine<T> {
    fn run(&self) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn stop(&self) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn terminate(&self) -> Lib3hResult<()> {
        // FIXME
        Ok(())
    }
    fn advertise(&self) -> String {
        self.transport_gateway.advertise()
    }

    /// Add incoming Lib3hClientProtocol message in FIFO
    fn post(&mut self, client_msg: Lib3hClientProtocol) -> Lib3hResult<()> {
        // println!("[t] RealEngine.post(): {:?}", client_msg);
        self.inbox.push_back(client_msg);
        Ok(())
    }

    /// Process Lib3hClientProtocol message inbox and
    /// output a list of Lib3hServerProtocol messages for Core to handle
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        // println!("[t] RealEngine.process()");
        // Process all received Lib3hClientProtocol messages from Core
        let (did_work, mut outbox) = self.process_inbox()?;
        // Process the transport layer
        let _ = self.process_transport_gateway()?;
        // Process all space dhts
        let p2p_output = self.process_space_gateways()?;
        // Process all generated P2pProtocol messages
        let mut output = self.process_p2p(&p2p_output)?;
        outbox.append(&mut output);
        // Done
        Ok((did_work, outbox))
    }
}

/// Private
impl<T: Transport> RealEngine<T> {
    /// Progressively serve every Lib3hClientProtocol received in inbox
    fn process_inbox(&mut self) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        let mut outbox = Vec::new();
        let mut did_work = false;
        loop {
            let client_msg = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            let (success, mut output) = self.serve_Lib3hProtocol(client_msg)?;
            if success {
                did_work = success;
            }
            outbox.append(&mut output);
        }
        Ok((did_work, outbox))
    }

    /// Progressively serve every P2pProtocol received in inbox
    fn process_transport_gateway(&mut self) -> Lib3hResult<DidWork> {
        let (did_work, p2p_list) = self.transport_gateway.do_process()?;
        if !did_work {
            return Ok(false);
        }
        for p2p_msg in p2p_list {
            self.serve_P2pProtocol(&p2p_msg)?;
        }
        Ok(true)
    }

    /// Process all space gateways
    fn process_space_gateways(&mut self) -> Lib3hResult<Vec<P2pProtocol>> {
        // Process all space gateways and store 'generated' P2pProtocol messages.
        let mut output = Vec::new();
        for (_space_address, space_gateway) in self.space_gateway_map.iter_mut() {
            let (did_work, mut p2p_list) = space_gateway.do_process()?;
            if did_work {
                output.append(&mut p2p_list);
            }
        }
        Ok(output)
    }
    /// Process all space gateways
    fn process_p2p(&mut self, input: &Vec<P2pProtocol>) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        // Serve all new P2pProtocols
        let mut output = Vec::new();
        for p2p_msg in input {
            let mut evt_output = self.serve_P2pProtocol(p2p_msg)?;
            output.append(&mut evt_output);
        }
        Ok(output)
    }
    /// Serve a transportEvent sent to us.
    /// Return a list of TransportEvents for us to process.
    // FIXME
    fn serve_P2pProtocol(
        &mut self,
        p2p_msg: &P2pProtocol,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        let outbox = Vec::new();
        match p2p_msg {
            P2pProtocol::Gossip => {
                // FIXME
            }
            P2pProtocol::DirectMessage => {
                // FIXME
            }
            P2pProtocol::FetchData => {
                // FIXME
            }
            P2pProtocol::FetchDataResponse => {
                // FIXME
            }
        };
        Ok(outbox)
    }

    /// Process a Lib3hClientProtocol message sent to us (by Core)
    /// Side effects: Might add other messages to sub-components' inboxes.
    /// Return a list of Lib3hServerProtocol messages to send back to core or others?
    fn serve_Lib3hProtocol(
        &mut self,
        client_msg: Lib3hClientProtocol,
    ) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        println!("[d] >>>> '{}' recv: {:?}", self.name.clone(), client_msg);
        let mut outbox = Vec::new();
        let did_work = true;
        // Note: use same order as the enum
        match client_msg {
            Lib3hClientProtocol::SuccessResult(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::FailureResult(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::Connect(msg) => {
                self.transport_gateway.connect(&msg.peer_transport)?;
            }
            Lib3hClientProtocol::JoinSpace(msg) => {
                let output = self.serve_JoinSpace(&msg)?;
                outbox.push(output);
            }
            Lib3hClientProtocol::LeaveSpace(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::SendDirectMessage(msg) => {
                if let Err(res) = self.has_space_or_fail(msg.space_address, msg.from_agent_id, msg.request_id, None) {
                    outbox.push(res);
                } else {
                    // Post a TransportCommand::Send request to the space gateway
                    let space_gateway = self.space_gateway_map.get(&(msg.space_address, msg.from_agent_id)).unwrap();
                    space_gateway.post(TransportCommand::Send([msg.to_agent_id], msg.content))?;
                }
            }
            Lib3hClientProtocol::HandleSendDirectMessageResult(msg) => {
                if let Err(res) = self.has_space_or_fail(msg.space_address, msg.from_agent_id, msg.request_id, Some(msg.to_agent_id)) {
                    outbox.push(res);
                } else {
                    // Post a TransportCommand::Send request to the space gateway
                    let space_gateway = self.space_gateway_map.get(&(msg.space_address, msg.from_agent_id)).unwrap();
                    space_gateway.post(TransportCommand::Send([msg.to_agent_id], msg.content))?;
                }
            }
            Lib3hClientProtocol::FetchEntry(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::HandleFetchEntryResult(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::PublishEntry(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::QueryEntry(msg) => {
                if let Err(res) = self.has_space_or_fail(msg.space_address, msg.requester_agent_id, msg.request_id, None) {
                    outbox.push(res);
                } else {
                    // Post a DhtEvent::FetchData request to the space gateway
                    let space_gateway = self.space_gateway_map.get(&(msg.space_address, msg.requester_agent_id)).unwrap();
                    let msg = DataFetchData {
                        msg_id: msg.request_id,
                        data_address: msg.entry_address,
                    };
                    space_gateway.post(DhtEvent::FetchData(msg));
                }
            }
            Lib3hClientProtocol::HandleQueryEntryResult(_msg) => {
                // FIXME
            }
            // Our request for the publish_list has returned
            Lib3hClientProtocol::HandleGetAuthoringEntryListResult(_msg) => {
                // FIXME
            }
            // Our request for the hold_list has returned
            Lib3hClientProtocol::HandleGetGossipingEntryListResult(_msg) => {
                // FIXME
            }
        }
        Ok((did_work, outbox))
    }

    /// Create a gateway for this agent in this space, if not already part of it.
    fn serve_JoinSpace(&mut self, join_msg: &SpaceData) -> Lib3hResult<Lib3hServerProtocol> {
        let player_id = (join_msg.space_address.clone(), join_msg.agent_id.clone());
        let mut res = ResultData {
            request_id: join_msg.request_id.clone(),
            space_address: join_msg.space_address.clone(),
            to_agent_id: join_msg.agent_id.clone(),
            result_info: vec![],
        };
        if self.space_gateway_map.contains_key(&player_id) {
            res.result_info = "Already tracked".to_string().into_bytes();
            return Ok(Lib3hServerProtocol::FailureResult(res));
        }
        self.space_gateway_map
            .insert(player_id.clone(), P2pGateway::new_with_space());
        let space_gateway = self.space_gateway_map.get_mut(&player_id).unwrap();
        Dht::post(
            space_gateway,
            DhtEvent::PeerHoldRequest(PeerHoldRequestData {
                peer_address: "FIXME".to_string(), // msg.agent_id,
                transport: self.transport_gateway.id(),
                timestamp: 42,
            }),
        )?;
        Ok(Lib3hServerProtocol::SuccessResult(res))
    }

    fn has_space_or_fail(&self, space_address: &str, agent_id: &str, request_id: &str, maybe_sender_agent_id: Option<&str>) -> Result<(), Lib3hServerProtocol> {
        let has_space = self.space_gateway_map.contains_key(&(space_address, agent_id));
        if has_space { return Ok(()) }
        let to_agent_id =  maybe_sender_agent_id.unwrap_or(agent_id);
        let res = GenericResultData {
            request_id,
            space_address,
            to_agent_id,
            result_info: format!("Agent {} does not track space {}", agent_id, space_address).to_bytes().to_vec(),
        };
        Err(Lib3hServerProtocol::FailureResult(res))
    }
}
