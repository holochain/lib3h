#![allow(non_snake_case)]

//#[cfg(test)]
use crate::transport::memory_mock::transport_memory::TransportMemory;
use std::collections::{HashMap, VecDeque};

use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol, Address, AddressRef, DidWork, Lib3hResult,
};
use rmp_serde::{Deserializer, Serializer};
use crate::{
    dht::{
        dht_protocol::{self, *},
        dht_trait::Dht,
        rrdht::RrDht,
    },
    gateway::p2p_gateway::P2pGateway,
    transport::{protocol::*, transport_trait::Transport},
    transport_space::TransportSpace,
    transport_wss::TransportWss,
    engine::{
        self::*,
        network_layer, space_layer,
        p2p_protocol::P2pProtocol,
    }
};

/// Lib3h's 'real mode' as a NetworkEngine
pub struct RealEngine<'t, T: Transport, D: DHT> {
    /// Config settings
    pub(crate) _config: RealEngineConfig,
    /// FIFO of Lib3hClientProtocol messages received from Core
    pub(crate) inbox: VecDeque<Lib3hClientProtocol>,
    /// Identifier
    pub(crate) name: String,
    /// P2p gateway for the transport layer,
    pub(crate) network_gateway: P2pGateway<'t, T, D>,
    /// Map of P2p gateway per Space+Agent
    pub(crate) space_gateway_map: HashMap<ChainId, P2pGateway<'t, P2pGateway<'t, T, D>, D>>,
}

impl RealEngine<TransportWss<std::net::TcpStream>, RrDht> {
    /// Constructor
    pub fn new(config: RealEngineConfig, name: &str) -> Lib3hResult<Self> {
        let mut network_gateway = P2pGateway::new_with_wss();
        network_gateway.bind("FIXME")?;
        Ok(RealEngine {
            _config: config,
            inbox: VecDeque::new(),
            name: name.to_string(),
            network_gateway: network_gateway,
            space_gateway_map: HashMap::new(),
        })
    }
}

/// Constructor
//#[cfg(test)]
impl RealEngine<TransportMemory, RrDht> {
    pub fn new_mock(config: RealEngineConfig, name: &str) -> Lib3hResult<Self> {
        let mut transport_gateway = P2pGateway::new_with_memory(name);
        transport_gateway.bind("FIXME")?;
        Ok(RealEngine {
            _config: config,
            inbox: VecDeque::new(),
            name: name.to_string(),
            network_gateway: transport_gateway,
            space_gateway_map: HashMap::new(),
        })
    }
}

impl<T: Transport, D: DHT> NetworkEngine for RealEngine<T, D> {
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
        let (did_work, mut outbox) = self.process_inbox()?;
        // Process the network layer
        let (net_did_work, mut net_outbox) = self.process_network_gateway()?;
        outbox.append(&mut net_outbox);
        // Process the space layer
        let p2p_output = self.process_space_gateways()?;
        // Done
        Ok((did_work, outbox))
    }
}

/// Private
impl<'t, T: Transport, D: DHT> RealEngine<'t, T, D> {
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
                self.network_gateway.connect(&msg.peer_transport)?;
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
                    Some(space_gateway) => {
                        let transport_id =
                            std::string::String::from_utf8_lossy(&msg.to_agent_id).into_owned();
                        // Change into P2pProtocol
                        let net_msg = P2pProtocol::DirectMessage(msg);
                        // Serialize
                        let mut payload = Vec::new();
                        net_msg.serialize(&mut Serializer::new(&mut payload)).unwrap();
                        // Send
                        space_gateway.send(&[transport_id], payload)?;
                    },
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
                    Some(space_gateway) => {
                        let transport_id =
                            std::string::String::from_utf8_lossy(&msg.to_agent_id).into_owned();
                        // Change into P2pProtocol
                        let net_msg = P2pProtocol::DirectMessageResult(msg);
                        // Serialize
                        let mut payload = Vec::new();
                        net_msg.serialize(&mut Serializer::new(&mut payload)).unwrap();
                        // Send
                        space_gateway.send(&[transport_id], payload)?;
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
                    &format!("PublishEntry_{}", msg.entry.entry_address),
                    None,
                );
                match maybe_space {
                    Err(res) => outbox.push(res),
                    Some(mut space_gateway) => {
                        // Post BroadcastEntry command
                        let cmd = DhtCommand::BroadcastEntry(msg.entry);
                        Dht::post(&space_gateway, cmd)?;
                    }
                }
            }
            Lib3hClientProtocol::HoldEntry(aspect) => {
                let maybe_space = self.get_space_or_fail(
                    &msg.space_address,
                    &msg.provider_agent_id,
                    &format!("HoldEntry_{}", msg.entry.entry_address),
                    None,
                );
                match maybe_space {
                    Err(res) => outbox.push(res),
                    Some(mut space_gateway) => {
                        // Post BroadcastEntry command
                        let cmd = DhtCommand::HoldEntry(msg.entry);
                        Dht::post(&space_gateway, cmd)?;
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
                    Some(mut space_gateway) => {
                        // Post BroadcastEntry command
                        let msg = dht_protocol::FetchEntryData {
                            msg_id: msg.request_id,
                            entry_address: msg.entry_address,
                        };
                        Dht::post(space_gateway, DhtCommand::FetchEntry(msg))?;
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
        let player_id = (join_msg.space_address.clone(), join_msg.agent_id.clone());
        let mut res = GenericResultData {
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
            .insert(player_id.clone(), P2pGateway::new_with_space(&self.network_gateway, &join_msg.space_address));
        let space_gateway = self.space_gateway_map.get_mut(&player_id).unwrap();
        Dht::post(
            space_gateway,
            DhtCommand::HoldPeer(PeerData {
                peer_address: "FIXME".to_string(), // msg.agent_id,
                transport: self.network_gateway.id(),
                timestamp: 42,
            }),
        )?;
        Ok(Lib3hServerProtocol::SuccessResult(res))
    }

    fn get_space_or_fail(
        &self,
        space_address: &AddressRef,
        agent_id: &AddressRef,
        request_id: &str,
        maybe_sender_agent_id: Option<&AddressRef>,
    ) -> Result<&P2pGateway<'t, P2pGateway<'t, T, D>, D>, Lib3hServerProtocol> {
        let maybe_space = self
            .space_gateway_map
            .get(&(space_address.to_owned(), agent_id.to_owned()));
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
