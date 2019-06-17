#![allow(non_snake_case)]

//#[cfg(test)]
use crate::transport::memory_mock::transport_memory::TransportMemory;
use std::collections::{HashMap, VecDeque};

use crate::{
    dht::{
        dht_protocol::{self, *},
        dht_trait::Dht,
        rrdht::RrDht,
    },
    engine::{p2p_protocol::P2pProtocol, RealEngine, RealEngineConfig},
    gateway::P2pGateway,
    transport::transport_trait::Transport,
    transport_wss::TransportWss,
};
use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol, AddressRef, DidWork, Lib3hResult,
};
use rmp_serde::Serializer;
use serde::Serialize;
use std::{cell::RefCell, rc::Rc};

impl RealEngine<TransportWss<std::net::TcpStream>, RrDht> {
    /// Constructor
    pub fn new(config: RealEngineConfig, name: &str) -> Lib3hResult<Self> {
        let network_transport = Rc::new(RefCell::new(TransportWss::with_std_tcp_stream()));
        let network_gateway = Rc::new(RefCell::new(P2pGateway::new(network_transport)));
        network_gateway.borrow_mut().bind("FIXME")?;
        Ok(RealEngine {
            _config: config,
            inbox: VecDeque::new(),
            name: name.to_string(),
            network_gateway,
            space_gateway_map: HashMap::new(),
        })
    }
}

/// Constructor
//#[cfg(test)]
impl RealEngine<TransportMemory, RrDht> {
    pub fn new_mock(config: RealEngineConfig, name: &str) -> Lib3hResult<Self> {
        let network_transport = Rc::new(RefCell::new(TransportMemory::new()));
        let network_gateway = Rc::new(RefCell::new(P2pGateway::new(network_transport)));
        let binding = network_gateway
            .borrow_mut()
            .bind(name)
            .expect("TransportMemory.bind() failed. url/name might not be unique?");
        network_gateway.borrow_mut().set_advertise(&binding);
        Ok(RealEngine {
            _config: config,
            inbox: VecDeque::new(),
            name: name.to_string(),
            network_gateway,
            space_gateway_map: HashMap::new(),
        })
    }
}

impl<T: Transport, D: Dht> NetworkEngine for RealEngine<T, D> {
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
        self.network_gateway
            .borrow()
            .advertise()
            .expect("Should always have an advertise")
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
        println!("[t] RealEngine.process()");
        // Process all received Lib3hClientProtocol messages from Core
        let (inbox_did_work, mut outbox) = self.process_inbox()?;
        // Process the network layer
        let (net_did_work, mut net_outbox) = self.process_network_gateway()?;
        outbox.append(&mut net_outbox);
        // Process the space layer
        let mut p2p_output = self.process_space_gateways()?;
        outbox.append(&mut p2p_output);
        // Done
        Ok((inbox_did_work || net_did_work, outbox))
    }
}

/// Private
impl<T: Transport, D: Dht> RealEngine<T, D> {
    /// Progressively serve every Lib3hClientProtocol received in inbox
    fn process_inbox(&mut self) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        let mut outbox = Vec::new();
        let mut did_work = false;
        loop {
            let client_msg = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            let (success, mut output) = self.serve_Lib3hClientProtocol(client_msg)?;
            if success {
                did_work = success;
            }
            outbox.append(&mut output);
        }
        Ok((did_work, outbox))
    }

    /// Process a Lib3hClientProtocol message sent to us (by Core)
    /// Side effects: Might add other messages to sub-components' inboxes.
    /// Return a list of Lib3hServerProtocol messages to send back to core or others?
    fn serve_Lib3hClientProtocol(
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
                self.network_gateway
                    .borrow_mut()
                    .connect(&msg.peer_transport)?;
            }
            Lib3hClientProtocol::JoinSpace(msg) => {
                let output = self.serve_JoinSpace(&msg)?;
                outbox.push(output);
            }
            Lib3hClientProtocol::LeaveSpace(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::SendDirectMessage(msg) => {
                let maybe_space = self.get_space_or_fail(
                    &msg.space_address,
                    &msg.from_agent_id,
                    &msg.request_id,
                    None,
                );
                match maybe_space {
                    Err(res) => outbox.push(res),
                    Ok(space_gateway) => {
                        let transport_id =
                            std::string::String::from_utf8_lossy(&msg.to_agent_id).into_owned();
                        // Change into P2pProtocol
                        let net_msg = P2pProtocol::DirectMessage(msg);
                        // Serialize
                        let mut payload = Vec::new();
                        net_msg
                            .serialize(&mut Serializer::new(&mut payload))
                            .unwrap();
                        // Send
                        space_gateway.send(&[transport_id.as_str()], &payload)?;
                    }
                }
            }
            Lib3hClientProtocol::HandleSendDirectMessageResult(msg) => {
                let maybe_space = self.get_space_or_fail(
                    &msg.space_address,
                    &msg.from_agent_id,
                    &msg.request_id,
                    Some(&msg.to_agent_id),
                );
                match maybe_space {
                    Err(res) => outbox.push(res),
                    Ok(space_gateway) => {
                        let transport_id =
                            std::string::String::from_utf8_lossy(&msg.to_agent_id).into_owned();
                        // Change into P2pProtocol
                        let net_msg = P2pProtocol::DirectMessageResult(msg);
                        // Serialize
                        let mut payload = Vec::new();
                        net_msg
                            .serialize(&mut Serializer::new(&mut payload))
                            .unwrap();
                        // Send
                        space_gateway.send(&[transport_id.as_str()], &payload)?;
                    }
                }
            }
            Lib3hClientProtocol::FetchEntry(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::HandleFetchEntryResult(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::PublishEntry(msg) => {
                let maybe_space = self.get_space_or_fail(
                    &msg.space_address,
                    &msg.provider_agent_id,
                    &format!("PublishEntry_{:?}", msg.entry.entry_address),
                    None,
                );
                match maybe_space {
                    Err(res) => outbox.push(res),
                    Ok(space_gateway) => {
                        // Post BroadcastEntry command
                        let cmd = DhtCommand::BroadcastEntry(msg.entry);
                        space_gateway.post_dht(cmd)?;
                        // Dht::post(&mut space_gateway, cmd)?;
                    }
                }
            }
            Lib3hClientProtocol::HoldEntry(msg) => {
                let maybe_space = self.get_space_or_fail(
                    &msg.space_address,
                    &msg.provider_agent_id,
                    &format!("HoldEntry_{:?}", msg.entry.entry_address),
                    None,
                );
                match maybe_space {
                    Err(res) => outbox.push(res),
                    Ok(space_gateway) => {
                        // Post BroadcastEntry command
                        let cmd = DhtCommand::HoldEntry(msg.entry);
                        space_gateway.post_dht(cmd)?;
                        // Dht::post(&mut space_gateway, cmd)?;
                    }
                }
            }
            Lib3hClientProtocol::QueryEntry(msg) => {
                let maybe_space = self.get_space_or_fail(
                    &msg.space_address,
                    &msg.requester_agent_id,
                    &msg.request_id,
                    None,
                );
                match maybe_space {
                    Err(res) => outbox.push(res),
                    Ok(space_gateway) => {
                        // Post BroadcastEntry command
                        let msg = dht_protocol::FetchEntryData {
                            msg_id: msg.request_id,
                            entry_address: msg.entry_address,
                        };
                        let cmd = DhtCommand::FetchEntry(msg);
                        space_gateway.post_dht(cmd)?;
                        // Dht::post(&mut space_gateway, cmd)?;
                    }
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
        let chain_id = (join_msg.space_address.clone(), join_msg.agent_id.clone());
        let mut res = GenericResultData {
            request_id: join_msg.request_id.clone(),
            space_address: join_msg.space_address.clone(),
            to_agent_id: join_msg.agent_id.clone(),
            result_info: vec![],
        };
        if self.space_gateway_map.contains_key(&chain_id) {
            res.result_info = "Already tracked".to_string().into_bytes();
            return Ok(Lib3hServerProtocol::FailureResult(res));
        }
        let new_space_gateway =
            P2pGateway::new_with_space(Rc::clone(&self.network_gateway), &join_msg.space_address);
        self.space_gateway_map
            .insert(chain_id.clone(), new_space_gateway);
        let space_gateway = self.space_gateway_map.get_mut(&chain_id).unwrap();
        Dht::post(
            space_gateway,
            DhtCommand::HoldPeer(PeerData {
                peer_address: "FIXME".to_string(), // msg.agent_id,
                transport: self.network_gateway.borrow().id(),
                timestamp: 42,
            }),
        )?;
        Ok(Lib3hServerProtocol::SuccessResult(res))
    }

    fn get_space_or_fail(
        &mut self,
        space_address: &AddressRef,
        agent_id: &AddressRef,
        request_id: &str,
        maybe_sender_agent_id: Option<&AddressRef>,
    ) -> Result<&mut P2pGateway<P2pGateway<T, D>, RrDht>, Lib3hServerProtocol> {
        let maybe_space = self
            .space_gateway_map
            .get_mut(&(space_address.to_owned(), agent_id.to_owned()));
        if let Some(space_gateway) = maybe_space {
            return Ok(space_gateway);
        }
        let to_agent_id = maybe_sender_agent_id.unwrap_or(agent_id);
        let res = GenericResultData {
            request_id: request_id.to_string(),
            space_address: space_address.to_owned(),
            to_agent_id: to_agent_id.to_owned(),
            result_info: format!(
                "Agent {} does not track space {}",
                std::string::String::from_utf8_lossy(&agent_id).into_owned(),
                std::string::String::from_utf8_lossy(&space_address).into_owned(),
            )
            .as_bytes()
            .to_vec(),
        };
        Err(Lib3hServerProtocol::FailureResult(res))
    }
}
