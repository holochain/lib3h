#![allow(non_snake_case)]

//#[cfg(test)]
use crate::transport::memory_mock::transport_memory::TransportMemory;
use std::collections::{HashMap, HashSet, VecDeque};
use url::Url;

use crate::{
    dht::{
        dht_protocol::{self, *},
        dht_trait::{Dht, DhtConfig, DhtFactory},
    },
    engine::{p2p_protocol::P2pProtocol, RealEngine, RealEngineConfig, TransportKeys},
    gateway::P2pGateway,
    transport::{protocol::TransportCommand, transport_trait::Transport},
    transport_wss::TransportWss,
};
use lib3h_crypto_api::{Buffer, CryptoSystem};
use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol, AddressRef, DidWork, Lib3hResult,
};
use rmp_serde::Serializer;
use serde::Serialize;
use std::{cell::RefCell, rc::Rc};

impl<SecBuf: Buffer, Crypto: CryptoSystem> TransportKeys<SecBuf, Crypto> {
    pub fn new() -> Lib3hResult<Self> {
        let hcm0 = hcid::HcidEncoding::with_kind("hcm0")?;
        let mut public_key = vec![0; Crypto::SIGN_PUBLIC_KEY_BYTES];
        let mut secret_key = SecBuf::new(Crypto::SIGN_SECRET_KEY_BYTES)?;
        Crypto::sign_keypair(&mut public_key, &mut secret_key)?;
        Ok(Self {
            transport_id: hcm0.encode(&public_key)?,
            transport_public_key: public_key,
            transport_secret_key: secret_key,
            phantom_crypto: std::marker::PhantomData,
        })
    }
}

impl<D: Dht, SecBuf: Buffer, Crypto: CryptoSystem>
    RealEngine<TransportWss<std::net::TcpStream>, D, SecBuf, Crypto>
{
    /// Constructor
    pub fn new(
        config: RealEngineConfig,
        name: &str,
        dht_factory: DhtFactory<D>,
    ) -> Lib3hResult<Self> {
        let network_transport = Rc::new(RefCell::new(TransportWss::with_std_tcp_stream()));
        let binding = network_transport.borrow_mut().bind(&config.bind_url)?;
        let transport_keys = TransportKeys::new()?;
        let dht_config = DhtConfig {
            this_peer_address: transport_keys.transport_id.clone(),
            this_peer_uri: binding,
            custom: config.dht_custom_config.clone(),
        };
        let network_gateway = Rc::new(RefCell::new(P2pGateway::new(
            "__physical_network__",
            Rc::clone(&network_transport),
            dht_factory,
            &dht_config,
        )));
        Ok(RealEngine {
            config: config,
            inbox: VecDeque::new(),
            name: name.to_string(),
            dht_factory,
            network_transport,
            network_gateway,
            network_connections: HashSet::new(),
            space_gateway_map: HashMap::new(),
            transport_keys,
        })
    }
}

/// Constructor
//#[cfg(test)]
impl<D: Dht, SecBuf: Buffer, Crypto: CryptoSystem> RealEngine<TransportMemory, D, SecBuf, Crypto> {
    pub fn new_mock(
        config: RealEngineConfig,
        name: &str,
        dht_factory: DhtFactory<D>,
    ) -> Lib3hResult<Self> {
        // Create TransportMemory as the network transport
        let network_transport = Rc::new(RefCell::new(TransportMemory::new()));
        // Bind & create DhtConfig
        let binding = network_transport
            .borrow_mut()
            .bind(&config.bind_url)
            .expect("TransportMemory.bind() failed. bind-url might not be unique?");
        let dht_config = DhtConfig {
            this_peer_address: format!("{}_tId", name),
            this_peer_uri: binding,
            custom: config.dht_custom_config.clone(),
        };
        // Create network gateway
        let network_gateway = Rc::new(RefCell::new(P2pGateway::new(
            "__memory_network__",
            Rc::clone(&network_transport),
            dht_factory,
            &dht_config,
        )));
        debug!(
            "New MOCK RealEngine {} -> {:?}",
            name,
            network_gateway.borrow().this_peer()
        );
        Ok(RealEngine {
            config: config,
            inbox: VecDeque::new(),
            name: name.to_string(),
            dht_factory,
            network_transport,
            network_gateway,
            network_connections: HashSet::new(),
            space_gateway_map: HashMap::new(),
            transport_keys: TransportKeys::new()?,
        })
    }
}

impl<T: Transport, D: Dht, SecBuf: Buffer, Crypto: CryptoSystem> NetworkEngine
    for RealEngine<T, D, SecBuf, Crypto>
{
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

    fn advertise(&self) -> Url {
        self.network_gateway
            .borrow()
            .this_peer()
            .peer_uri
            .to_owned()
    }

    /// Add incoming Lib3hClientProtocol message in FIFO
    fn post(&mut self, client_msg: Lib3hClientProtocol) -> Lib3hResult<()> {
        debug!("[t] RealEngine.post(): {:?}", client_msg);
        self.inbox.push_back(client_msg);
        Ok(())
    }

    /// Process Lib3hClientProtocol message inbox and
    /// output a list of Lib3hServerProtocol messages for Core to handle
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        debug!("\n[t] {} - RealEngine.process() START", self.name);
        // Process all received Lib3hClientProtocol messages from Core
        let (inbox_did_work, mut outbox) = self.process_inbox()?;
        // Process the network layer
        let (net_did_work, mut net_outbox) = self.process_network_gateway()?;
        outbox.append(&mut net_outbox);
        // Process the space layer
        let mut p2p_output = self.process_space_gateways()?;
        outbox.append(&mut p2p_output);
        debug!("[t] {} - RealEngine.process() END\n", self.name);
        // Done
        Ok((inbox_did_work || net_did_work, outbox))
    }
}

/// Private
impl<T: Transport, D: Dht, SecBuf: Buffer, Crypto: CryptoSystem> RealEngine<T, D, SecBuf, Crypto> {
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
        debug!("[d] {} serving: {:?}", self.name.clone(), client_msg);
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
                // Convert into TransportCommand & post to network gateway
                let cmd = TransportCommand::Connect(msg.peer_uri);
                Transport::post(&mut *self.network_gateway.borrow_mut(), cmd)?;
            }
            Lib3hClientProtocol::JoinSpace(msg) => {
                let output = self.serve_JoinSpace(&msg)?;
                outbox.push(output);
            }
            Lib3hClientProtocol::LeaveSpace(_msg) => {
                // FIXME
            }
            Lib3hClientProtocol::SendDirectMessage(msg) => {
                let my_name = self.name.clone();
                let maybe_space = self.get_space_or_fail(
                    &msg.space_address,
                    &msg.from_agent_id,
                    &msg.request_id,
                    None,
                );
                match maybe_space {
                    Err(res) => outbox.push(res),
                    Ok(space_gateway) => {
                        let connection_id =
                            std::string::String::from_utf8_lossy(&msg.to_agent_id).into_owned();
                        debug!("[d] {} -- connection_id: {:?}", my_name, connection_id);
                        // Change into P2pProtocol
                        let net_msg = P2pProtocol::DirectMessage(msg);
                        // Serialize
                        let mut payload = Vec::new();
                        net_msg
                            .serialize(&mut Serializer::new(&mut payload))
                            .unwrap();
                        // Send
                        debug!(
                            "[t] {} sending payload to transport id {}",
                            my_name, connection_id
                        );
                        space_gateway.send(&[connection_id.as_str()], &payload)?;
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
                        let connection_id =
                            std::string::String::from_utf8_lossy(&msg.to_agent_id).into_owned();
                        // Change into P2pProtocol
                        let net_msg = P2pProtocol::DirectMessageResult(msg);
                        // Serialize
                        let mut payload = Vec::new();
                        net_msg
                            .serialize(&mut Serializer::new(&mut payload))
                            .unwrap();
                        // Send
                        space_gateway.send(&[connection_id.as_str()], &payload)?;
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
    /// Must not already be part of this space.
    fn serve_JoinSpace(&mut self, join_msg: &SpaceData) -> Lib3hResult<Lib3hServerProtocol> {
        // Prepare response
        let mut res = GenericResultData {
            request_id: join_msg.request_id.clone(),
            space_address: join_msg.space_address.clone(),
            to_agent_id: join_msg.agent_id.clone(),
            result_info: vec![],
        };
        // Bail if space already joined by agent
        let chain_id = (join_msg.space_address.clone(), join_msg.agent_id.clone());
        if self.space_gateway_map.contains_key(&chain_id) {
            res.result_info = "Already joined space".to_string().into_bytes();
            return Ok(Lib3hServerProtocol::FailureResult(res));
        }
        // First create DhtConfig for space gateway
        let agent_id = std::string::String::from_utf8_lossy(&join_msg.agent_id).into_owned();
        let this_net_peer = self.network_gateway.borrow().this_peer().clone();
        let this_peer_transport =
            // TODO encapsulate this conversion logic
            Url::parse(format!("transport:{}", this_net_peer.peer_address.clone()).as_str()).unwrap();
        let dht_config = DhtConfig {
            this_peer_address: agent_id,
            this_peer_uri: this_peer_transport,
            custom: self.config.dht_custom_config.clone(),
        };
        // Create new space gateway for this ChainId
        let new_space_gateway = P2pGateway::new_with_space(
            Rc::clone(&self.network_gateway),
            &join_msg.space_address,
            self.dht_factory,
            &dht_config,
        );

        // HACK: Send JoinSpace to all known peers
        let space_address =
            std::string::String::from_utf8_lossy(&join_msg.space_address).into_owned();
        let peer = new_space_gateway.this_peer().to_owned();
        let mut payload = Vec::new();
        let p2p_msg = P2pProtocol::BroadcastJoinSpace(space_address.clone(), peer.clone());
        p2p_msg
            .serialize(&mut Serializer::new(&mut payload))
            .unwrap();
        debug!(
            "[t] {} - Broadcasting JoinSpace: {}, {}",
            self.name.clone(),
            space_address,
            peer.peer_address
        );
        self.network_gateway.borrow_mut().send_all(&payload).ok();
        // HACK END

        // Add it to space map
        self.space_gateway_map
            .insert(chain_id.clone(), new_space_gateway);
        // Have DHT broadcast our PeerData
        let space_gateway = self.space_gateway_map.get_mut(&chain_id).unwrap();
        Dht::post(
            space_gateway,
            DhtCommand::HoldPeer(PeerData {
                peer_address: dht_config.this_peer_address,
                peer_uri: dht_config.this_peer_uri,
                timestamp: 42, // FIXME
            }),
        )?;
        // Done
        Ok(Lib3hServerProtocol::SuccessResult(res))
    }

    fn get_space_or_fail(
        &mut self,
        space_address: &AddressRef,
        agent_id: &AddressRef,
        request_id: &str,
        maybe_sender_agent_id: Option<&AddressRef>,
    ) -> Result<&mut P2pGateway<P2pGateway<T, D>, D>, Lib3hServerProtocol> {
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
