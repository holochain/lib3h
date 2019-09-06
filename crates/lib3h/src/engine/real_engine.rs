#![allow(non_snake_case)]

//#[cfg(test)]
use crate::transport::memory_mock::transport_memory::TransportMemory;
use std::collections::{HashMap, HashSet, VecDeque};
use url::Url;

use super::RealEngineTrackerData;
use crate::{
    dht::{
        dht_protocol::{self, *},
        dht_trait::*,
    },
    engine::{
        p2p_protocol::P2pProtocol, RealEngine, RealEngineConfig, TransportKeys, NETWORK_GATEWAY_ID,
    },
    error::Lib3hResult,
    gateway::{GatewayWrapper, P2pGateway},
    track::Tracker,
    transport::{protocol::TransportCommand, TransportWrapper, TransportGhostAsLegacy, TransportLegacyAsGhost},
    transport_wss::TransportWss,
};
use lib3h_crypto_api::{Buffer, CryptoSystem};
use lib3h_protocol::{
    data_types::*, error::Lib3hProtocolResult, network_engine::NetworkEngine,
    protocol_client::Lib3hClientProtocol, protocol_server::Lib3hServerProtocol, Address, DidWork,
};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

impl TransportKeys {
    pub fn new(crypto: &dyn CryptoSystem) -> Lib3hResult<Self> {
        let hcm0 = hcid::HcidEncoding::with_kind("hcm0")?;
        let mut public_key: Box<dyn Buffer> = Box::new(vec![0; crypto.sign_public_key_bytes()]);
        let mut secret_key = crypto.buf_new_secure(crypto.sign_secret_key_bytes());
        crypto.sign_keypair(&mut public_key, &mut secret_key)?;
        Ok(Self {
            transport_id: hcm0.encode(&public_key)?,
            transport_public_key: public_key,
            transport_secret_key: secret_key,
        })
    }
}

impl<'engine, D: Dht> RealEngine<'engine, D> {
    /// Constructor with TransportWss
    pub fn new(
        crypto: Box<dyn CryptoSystem>,
        config: RealEngineConfig,
        name: &str,
        dht_factory: DhtFactory<D>,
    ) -> Lib3hResult<Self> {
        // Create Transport and bind
        let network_transport =
            TransportWrapper::new(TransportWss::with_std_tcp_stream(config.tls_config.clone()));
        let binding = network_transport.as_mut().bind(&config.bind_url)?;
        // Generate keys
        // TODO #209 - Check persistence first before generating
        let transport_keys = TransportKeys::new(crypto.as_crypto_system())?;
        // Generate DHT config and create network_gateway
        let dht_config = DhtConfig {
            this_peer_address: transport_keys.transport_id.clone(),
            this_peer_uri: binding,
            custom: config.dht_custom_config.clone(),
            gossip_interval: config.dht_gossip_interval,
            timeout_threshold: config.dht_timeout_threshold,
        };
        let network_gateway = GatewayWrapper::new(P2pGateway::new(
            NETWORK_GATEWAY_ID,
            network_transport.clone(),
            dht_factory,
            &dht_config,
        ));
        // Done
        Ok(RealEngine {
            crypto,
            config,
            inbox: VecDeque::new(),
            name: name.to_string(),
            dht_factory,
            request_track: Tracker::new("real_engine_", 2000),
            network_transport,
            network_gateway,
            network_connections: HashSet::new(),
            space_gateway_map: HashMap::new(),
            transport_keys,
            process_count: 0,
        })
    }
}

/// Constructor
//#[cfg(test)]
impl<'engine, D: Dht> RealEngine<'engine, D> {
    /// Constructor with TransportMemory
    pub fn new_mock(
        crypto: Box<dyn CryptoSystem>,
        config: RealEngineConfig,
        name: &str,
        dht_factory: DhtFactory<D>,
    ) -> Lib3hResult<Self> {
        // Create TransportMemory as the network transport
        let network_transport = TransportWrapper::new(TransportMemory::new());
        // Bind & create DhtConfig
        let binding = network_transport
            .as_mut()
            .bind(&config.bind_url)
            .expect("TransportMemory.bind() failed. bind-url might not be unique?");
        let dht_config = DhtConfig {
            this_peer_address: format!("{}_tId", name),
            this_peer_uri: binding,
            custom: config.dht_custom_config.clone(),
            gossip_interval: config.dht_gossip_interval,
            timeout_threshold: config.dht_timeout_threshold,
        };
        // Create network gateway
        let network_gateway = GatewayWrapper::new(P2pGateway::new(
            NETWORK_GATEWAY_ID,
            network_transport.clone(),
            dht_factory,
            &dht_config,
        ));
        debug!(
            "New MOCK RealEngine {} -> {:?}",
            name,
            network_gateway.as_ref().this_peer()
        );
        let transport_keys = TransportKeys::new(crypto.as_crypto_system())?;
        Ok(RealEngine {
            crypto,
            config,
            inbox: VecDeque::new(),
            name: name.to_string(),
            dht_factory,
            request_track: Tracker::new("real_engine_", 2000),
            network_transport,
            network_gateway,
            network_connections: HashSet::new(),
            space_gateway_map: HashMap::new(),
            transport_keys,
            process_count: 0,
        })
    }
}

impl<'engine, D: Dht> NetworkEngine for RealEngine<'engine, D> {
    /// User provided identifier for this engine
    fn name(&self) -> String {
        self.name.clone()
    }

    fn advertise(&self) -> Url {
        self.network_gateway
            .as_dht_ref()
            .this_peer()
            .peer_uri
            .to_owned()
    }

    /// Add incoming Lib3hClientProtocol message in FIFO
    fn post(&mut self, client_msg: Lib3hClientProtocol) -> Lib3hProtocolResult<()> {
        // trace!("RealEngine.post(): {:?}", client_msg);
        self.inbox.push_back(client_msg);
        Ok(())
    }

    /// Process Lib3hClientProtocol message inbox and
    /// output a list of Lib3hServerProtocol messages for Core to handle
    fn process(&mut self) -> Lib3hProtocolResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        self.process_count += 1;
        trace!("");
        trace!("{} - process() START - {}", self.name, self.process_count);
        // Process all received Lib3hClientProtocol messages from Core
        let (inbox_did_work, mut outbox) = self.process_inbox()?;
        // Process the network layer
        let (net_did_work, mut net_outbox) = self.process_network_gateway()?;
        outbox.append(&mut net_outbox);
        // Process the space layer
        let mut p2p_output = self.process_space_gateways()?;
        outbox.append(&mut p2p_output);
        trace!(
            "process() END - {} (outbox: {})\n",
            self.process_count,
            outbox.len(),
        );

        for (timeout_id, timeout_data) in self.request_track.process_timeouts() {
            error!("timeout {:?} {:?}", timeout_id, timeout_data);
        }

        // Done
        Ok((inbox_did_work || net_did_work, outbox))
    }
}

/// Drop
impl<'engine, D: Dht> Drop for RealEngine<'engine, D> {
    fn drop(&mut self) {
        self.shutdown().unwrap_or_else(|e| {
            warn!("Graceful shutdown failed: {}", e);
        });
    }
}

/// Private
impl<'engine, D: Dht> RealEngine<'engine, D> {
    /// Called on drop.
    /// Close all connections gracefully
    fn shutdown(&mut self) -> Lib3hResult<()> {
        let mut result: Lib3hResult<()> = Ok(());

        for space_gatway in self.space_gateway_map.values_mut() {
            let res = space_gatway.as_transport_mut().close_all();
            // Continue closing connections even if some failed
            if let Err(e) = res {
                if result.is_ok() {
                    result = Err(e.into());
                }
            }
        }
        // Done
        self.network_gateway
            .as_transport_mut()
            .close_all()
            .map_err(|e| {
                error!("Closing of some connection failed: {:?}", e);
                e
            })?;

        result
    }

    /// Progressively serve every Lib3hClientProtocol received in inbox
    fn process_inbox(&mut self) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        let mut outbox = Vec::new();
        let did_work = self.inbox.len() > 0;
        loop {
            let client_msg = match self.inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            let mut output = self.serve_Lib3hClientProtocol(client_msg)?;
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
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        debug!("{} serving: {:?}", self.name, client_msg);
        let mut outbox = Vec::new();
        // Note: use same order as the enum
        match client_msg {
            Lib3hClientProtocol::Shutdown => {
                // TODO
            }
            Lib3hClientProtocol::SuccessResult(_msg) => {
                // TODO #168
            }
            Lib3hClientProtocol::FailureResult(_msg) => {
                // TODO #168
            }
            Lib3hClientProtocol::Connect(msg) => {
                // Convert into TransportCommand & post to network gateway
                let cmd = TransportCommand::Connect(msg.peer_uri, msg.request_id);
                self.network_gateway.as_transport_mut().post(cmd)?;
            }
            Lib3hClientProtocol::JoinSpace(msg) => {
                let mut output = self.serve_JoinSpace(&msg)?;
                outbox.append(&mut output);
            }
            Lib3hClientProtocol::LeaveSpace(msg) => {
                let srv_msg = self.serve_LeaveSpace(&msg);
                outbox.push(srv_msg);
            }
            Lib3hClientProtocol::SendDirectMessage(msg) => {
                let srv_msg = self.serve_DirectMessage(msg, false);
                outbox.push(srv_msg);
            }
            Lib3hClientProtocol::HandleSendDirectMessageResult(msg) => {
                let srv_msg = self.serve_DirectMessage(msg, true);
                outbox.push(srv_msg);
            }
            Lib3hClientProtocol::FetchEntry(_msg) => {
                // TODO #169
            }
            // HandleFetchEntryResult:
            //   - From GetAuthoringList      : Convert to DhtCommand::BroadcastEntry
            //   - From DHT EntryDataRequested: Convert to DhtCommand::EntryDataResponse
            Lib3hClientProtocol::HandleFetchEntryResult(msg) => {
                let mut is_data_for_author_list = false;
                if self.request_track.has(&msg.request_id) {
                    match self.request_track.remove(&msg.request_id) {
                        Some(data) => match data {
                            RealEngineTrackerData::DataForAuthorEntry => {
                                is_data_for_author_list = true;
                            }
                            _ => (),
                        },
                        None => (),
                    };
                }
                let maybe_space = self.get_space_or_fail(
                    &msg.space_address,
                    &msg.provider_agent_id,
                    &msg.request_id,
                    None,
                );
                match maybe_space {
                    Err(res) => outbox.push(res),
                    Ok(space_gateway) => {
                        if is_data_for_author_list {
                            let cmd = DhtCommand::BroadcastEntry(msg.entry);
                            space_gateway.as_dht_mut().post(cmd)?;
                        } else {
                            let response = FetchDhtEntryResponseData {
                                msg_id: msg.request_id.clone(),
                                entry: msg.entry.clone(),
                            };
                            let cmd = DhtCommand::EntryDataResponse(response);
                            space_gateway.as_dht_mut().post(cmd)?;
                        }
                    }
                }
            }
            // PublishEntry: Broadcast on the space DHT
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
                        let cmd = DhtCommand::BroadcastEntry(msg.entry);
                        space_gateway.as_dht_mut().post(cmd)?;
                    }
                }
            }
            // HoldEntry: Core validated an entry/aspect and tells us its holding it.
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
                        let cmd = DhtCommand::HoldEntryAspectAddress(msg.entry);
                        space_gateway.as_dht_mut().post(cmd)?;
                    }
                }
            }
            // QueryEntry: Converting to DHT FetchEntry for now
            // TODO #169
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
                        let msg = dht_protocol::FetchDhtEntryData {
                            msg_id: msg.request_id,
                            entry_address: msg.entry_address,
                        };
                        let cmd = DhtCommand::FetchEntry(msg);
                        space_gateway.as_dht_mut().post(cmd)?;
                    }
                }
            }
            // HandleQueryEntryResult: Convert into DhtCommand::ProvideEntryResponse
            // TODO #169
            Lib3hClientProtocol::HandleQueryEntryResult(msg) => {
                let maybe_space = self.get_space_or_fail(
                    &msg.space_address,
                    &msg.responder_agent_id,
                    &msg.request_id,
                    None,
                );
                let mut de = Deserializer::new(&msg.query_result[..]);
                let maybe_entry: Result<EntryData, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                let entry = maybe_entry.expect("Deserialization should always work");
                match maybe_space {
                    Err(res) => outbox.push(res),
                    Ok(space_gateway) => {
                        let msg = dht_protocol::FetchDhtEntryResponseData {
                            msg_id: msg.request_id,
                            entry,
                        };
                        let cmd = DhtCommand::EntryDataResponse(msg);
                        space_gateway.as_dht_mut().post(cmd)?;
                    }
                }
            }
            // Our request for the publish_list has returned
            Lib3hClientProtocol::HandleGetAuthoringEntryListResult(msg) => {
                self.serve_Lib3hClientProtocol_HandleGetAuthoringEntryListResult(&mut outbox, msg)?;
            }
            // Our request for the hold_list has returned
            Lib3hClientProtocol::HandleGetGossipingEntryListResult(msg) => {
                self.serve_Lib3hClientProtocol_HandleGetGossipingEntryListResult(&mut outbox, msg)?;
            }
        }
        Ok(outbox)
    }

    fn serve_Lib3hClientProtocol_HandleGetAuthoringEntryListResult(
        &mut self,
        outbox: &mut Vec<Lib3hServerProtocol>,
        msg: EntryListData,
    ) -> Lib3hResult<()> {
        if !self.request_track.has(&msg.request_id) {
            error!("untracked HandleGetAuthoringEntryListResult");
        } else {
            match self.request_track.remove(&msg.request_id) {
                Some(data) => match data {
                    RealEngineTrackerData::GetAuthoringEntryList => (),
                    _ => error!("bad track type HandleGetAuthoringEntryListResult"),
                },
                None => error!("bad track type HandleGetAuthoringEntryListResult"),
            };
        }
        let mut request_list = Vec::new();
        let maybe_space = self.get_space_or_fail(
            &msg.space_address,
            &msg.provider_agent_id,
            &msg.request_id,
            None,
        );
        match maybe_space {
            Err(res) => outbox.push(res),
            Ok(space_gateway) => {
                let mut msg_data = FetchEntryData {
                    space_address: msg.space_address.clone(),
                    entry_address: "".into(),
                    request_id: "".into(),
                    provider_agent_id: msg.provider_agent_id.clone(),
                    aspect_address_list: None,
                };
                // Request every Entry from Core
                let mut count = 0;
                for (entry_address, aspect_address_list) in msg.address_map {
                    // Check aspects and only request entry with new aspects
                    let maybe_known_aspects = space_gateway.as_ref().get_aspects_of(&entry_address);
                    if let Some(known_aspects) = maybe_known_aspects {
                        if includes(&known_aspects, &aspect_address_list) {
                            continue;
                        }
                    }
                    count += 1;
                    msg_data.entry_address = entry_address.clone();
                    request_list.push(msg_data.clone());
                }
                debug!("HandleGetAuthoringEntryListResult: {}", count);
            }
        }
        for mut msg_data in request_list {
            msg_data.request_id = self.request_track.reserve();
            self.request_track.set(
                &msg_data.request_id,
                Some(RealEngineTrackerData::DataForAuthorEntry),
            );
            outbox.push(Lib3hServerProtocol::HandleFetchEntry(msg_data));
        }
        Ok(())
    }

    fn serve_Lib3hClientProtocol_HandleGetGossipingEntryListResult(
        &mut self,
        outbox: &mut Vec<Lib3hServerProtocol>,
        msg: EntryListData,
    ) -> Lib3hResult<()> {
        if !self.request_track.has(&msg.request_id) {
            error!("untracked HandleGetGossipingEntryListResult");
        } else {
            match self.request_track.remove(&msg.request_id) {
                Some(data) => match data {
                    RealEngineTrackerData::GetGossipingEntryList => (),
                    _ => error!("bad track type HandleGetGossipingEntryListResult"),
                },
                None => error!("bad track type HandleGetGossipingEntryListResult"),
            };
        }
        let maybe_space = self.get_space_or_fail(
            &msg.space_address,
            &msg.provider_agent_id,
            &msg.request_id,
            None,
        );
        match maybe_space {
            Err(res) => outbox.push(res),
            Ok(space_gateway) => {
                for (entry_address, aspect_address_list) in msg.address_map {
                    let mut aspect_list = Vec::new();
                    for aspect_address in aspect_address_list {
                        let fake_aspect = EntryAspectData {
                            aspect_address: aspect_address.clone(),
                            type_hint: String::new(),
                            aspect: vec![],
                            publish_ts: 0,
                        };
                        aspect_list.push(fake_aspect);
                    }
                    // Create "fake" entry, in the sense an entry with no actual content,
                    // but valid addresses.
                    let fake_entry = EntryData {
                        entry_address: entry_address.clone(),
                        aspect_list,
                    };
                    space_gateway
                        .as_dht_mut()
                        .post(DhtCommand::HoldEntryAspectAddress(fake_entry))?;
                }
            }
        }
        Ok(())
    }

    /// Create a gateway for this agent in this space, if not already part of it.
    /// Must not already be part of this space.
    fn serve_JoinSpace(&mut self, join_msg: &SpaceData) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
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
            return Ok(vec![Lib3hServerProtocol::FailureResult(res)]);
        }
        let mut output = Vec::new();
        output.push(Lib3hServerProtocol::SuccessResult(res));
        // First create DhtConfig for space gateway
        let agent_id: String = join_msg.agent_id.clone().into();
        let this_peer_transport_id_as_uri = {
            let gateway = self.network_gateway.as_ref();
            // TODO #175 - encapsulate this conversion logic
            Url::parse(format!("transportId:{}", gateway.this_peer().peer_address).as_str())
                .expect("can parse url")
        };
        let dht_config = DhtConfig {
            this_peer_address: agent_id,
            this_peer_uri: this_peer_transport_id_as_uri,
            custom: self.config.dht_custom_config.clone(),
            gossip_interval: self.config.dht_gossip_interval,
            timeout_threshold: self.config.dht_timeout_threshold,
        };

        // --- begin hack legacy->ghost->legacy --- //
        let sgit_ghost: TransportLegacyAsGhost<'engine> =
            TransportLegacyAsGhost::new(
                self.network_gateway.as_transport()
            );
        let sgit_legacy = TransportGhostAsLegacy::new(Box::new(sgit_ghost));
        let space_gateway_inner_transport = TransportWrapper::new(sgit_legacy);
        // ---  end hack legacy->ghost->legacy  --- //

        // Create new space gateway for this ChainId
        let new_space_gateway: GatewayWrapper<'engine> =
            GatewayWrapper::new(P2pGateway::new_with_space(
                space_gateway_inner_transport,
                &join_msg.space_address,
                self.dht_factory,
                &dht_config,
            ));

        // TODO #150 - Send JoinSpace to all known peers
        let space_address: String = join_msg.space_address.clone().into();
        let peer = new_space_gateway.as_ref().this_peer().to_owned();
        let mut payload = Vec::new();
        let p2p_msg = P2pProtocol::BroadcastJoinSpace(space_address.clone(), peer.clone());
        p2p_msg
            .serialize(&mut Serializer::new(&mut payload))
            .unwrap();
        trace!(
            "{} - Broadcasting JoinSpace: {}, {}",
            self.name,
            space_address,
            peer.peer_address,
        );
        self.network_gateway
            .as_transport_mut()
            .send_all(&payload)
            .ok();
        // TODO END

        // Add it to space map
        self.space_gateway_map
            .insert(chain_id.clone(), new_space_gateway);
        // Have DHT broadcast our PeerData
        let space_gateway = self.space_gateway_map.get_mut(&chain_id).unwrap();
        let this_peer = { space_gateway.as_ref().this_peer().clone() };
        space_gateway
            .as_dht_mut()
            .post(DhtCommand::HoldPeer(this_peer))?;
        // Send Get*Lists requests
        let mut list_data = GetListData {
            space_address: join_msg.space_address.clone(),
            provider_agent_id: join_msg.agent_id.clone(),
            request_id: self.request_track.reserve(),
        };
        self.request_track.set(
            &list_data.request_id,
            Some(RealEngineTrackerData::GetGossipingEntryList),
        );
        output.push(Lib3hServerProtocol::HandleGetGossipingEntryList(
            list_data.clone(),
        ));
        list_data.request_id = self.request_track.reserve();
        self.request_track.set(
            &list_data.request_id,
            Some(RealEngineTrackerData::GetAuthoringEntryList),
        );
        output.push(Lib3hServerProtocol::HandleGetAuthoringEntryList(list_data));
        // Done
        Ok(output)
    }

    fn serve_DirectMessage(
        &mut self,
        msg: DirectMessageData,
        is_response: bool,
    ) -> Lib3hServerProtocol {
        // Check if space is joined by sender
        let maybe_space = self.get_space_or_fail(
            &msg.space_address,
            &msg.from_agent_id,
            &msg.request_id,
            None,
        );
        // Return failure if not
        if let Err(failure_msg) = maybe_space {
            return failure_msg;
        }
        let space_gateway = maybe_space.unwrap();
        // Prepare response
        let mut response = GenericResultData {
            request_id: msg.request_id.clone(),
            space_address: msg.space_address.clone(),
            to_agent_id: msg.from_agent_id.clone(),
            result_info: vec![],
        };
        // Check if messaging self
        let peer_address = &space_gateway.as_ref().this_peer().peer_address.clone();
        let to_agent_id: String = msg.to_agent_id.clone().into();
        if peer_address == &to_agent_id {
            response.result_info = "Messaging self".as_bytes().to_vec();
            return Lib3hServerProtocol::FailureResult(response);
        }
        // Change into P2pProtocol
        let net_msg = if is_response {
            P2pProtocol::DirectMessageResult(msg.clone())
        } else {
            P2pProtocol::DirectMessage(msg.clone())
        };
        // Serialize payload
        let mut payload = Vec::new();
        net_msg
            .serialize(&mut Serializer::new(&mut payload))
            .unwrap();
        // Send
        let peer_address: String = msg.to_agent_id.clone().into();
        let res = space_gateway
            .as_transport_mut()
            .send(&[peer_address.as_str()], &payload);
        if let Err(e) = res {
            response.result_info = e.to_string().as_bytes().to_vec();
            return Lib3hServerProtocol::FailureResult(response);
        }
        Lib3hServerProtocol::SuccessResult(response)
    }

    /// Destroy gateway for this agent in this space, if part of it.
    /// Respond with FailureResult if space was not already joined.
    fn serve_LeaveSpace(&mut self, join_msg: &SpaceData) -> Lib3hServerProtocol {
        // Try remove
        let chain_id = (join_msg.space_address.clone(), join_msg.agent_id.clone());
        let res = self.space_gateway_map.remove(&chain_id);
        // Create response according to remove result
        let response = GenericResultData {
            request_id: join_msg.request_id.clone(),
            space_address: join_msg.space_address.clone(),
            to_agent_id: join_msg.agent_id.clone(),
            result_info: match res {
                None => "Agent is not part of the space".to_string().into_bytes(),
                Some(_) => vec![],
            },
        };
        // Done
        match res {
            None => Lib3hServerProtocol::FailureResult(response),
            Some(_) => Lib3hServerProtocol::SuccessResult(response),
        }
    }

    /// Get a space_gateway for the specified space+agent.
    /// If agent did not join that space, respond with a FailureResult instead.
    fn get_space_or_fail(
        &mut self,
        space_address: &Address,
        agent_id: &Address,
        request_id: &str,
        maybe_sender_agent_id: Option<&Address>,
    ) -> Result<GatewayWrapper<'engine>, Lib3hServerProtocol> {
        let maybe_space = self
            .space_gateway_map
            .get_mut(&(space_address.to_owned(), agent_id.to_owned()));
        if let Some(space_gateway) = maybe_space {
            return Ok(space_gateway.clone());
        }
        let to_agent_id = maybe_sender_agent_id.unwrap_or(agent_id);
        let res = GenericResultData {
            request_id: request_id.to_string(),
            space_address: space_address.to_owned(),
            to_agent_id: to_agent_id.to_owned(),
            result_info: format!(
                "Agent {} does not track space {}",
                &agent_id, &space_address,
            )
            .as_bytes()
            .to_vec(),
        };
        Err(Lib3hServerProtocol::FailureResult(res))
    }
}

/// Return true if all elements of list_b are found in list_a
fn includes(list_a: &[Address], list_b: &[Address]) -> bool {
    let set_a: HashSet<_> = list_a.iter().map(|addr| addr).collect();
    let set_b: HashSet<_> = list_b.iter().map(|addr| addr).collect();
    set_b.is_subset(&set_a)
}
