use detach::Detach;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::*, protocol::*, Address};
use std::collections::{HashMap, HashSet};

use super::RealEngineTrackerData;
use crate::{
    dht::{dht_config::DhtConfig, dht_protocol::*},
    engine::{
        p2p_protocol::P2pProtocol, ChainId, RealEngineConfig, TransportKeys, NETWORK_GATEWAY_ID,
    },
    error::{ErrorKind, Lib3hError, Lib3hResult},
    gateway::{wrapper::*, P2pGateway},
    track::Tracker,
    transport::{ConnectionId, TransportWrapper},
};
use lib3h_crypto_api::CryptoSystem;
use rmp_serde::Serializer;
use serde::Serialize;
use url::Url;

/// the context when making a request
#[derive(Clone)]
struct RequestContext {
    pub space_address: Address,
    pub agent_id: Address,
}

/// this is a generic parent wrapper for a GhostEngine.  This allows us to have
/// a mock GhostEngine for proving out the LegacyLib3h wrapper
pub type GhostEngineParentWrapper<Core, Context, Engine, EngineError> = GhostParentWrapper<
    Core,
    Context,
    Lib3hToClient,
    Lib3hToClientResponse,
    ClientToLib3h,
    ClientToLib3hResponse,
    EngineError,
    Engine,
>;

pub type ClientToLib3hMessage =
    GhostMessage<ClientToLib3h, Lib3hToClient, ClientToLib3hResponse, Lib3hError>;

pub type Lib3hToClientMessage =
    GhostMessage<Lib3hToClient, ClientToLib3h, Lib3hToClientResponse, Lib3hError>;

/*
// temporary mock gateway...
struct MockGateway {
    #[allow(dead_code)]
    space_address: Address,
    this_peer: PeerData,
    dht: DhtEndpointWithContext
}
impl MockGateway {
    fn new((space_address, peer_id): (Address, Address)) -> Self {
        MockGateway {
            space_address: space_address.clone(),
            this_peer: PeerData {
                peer_address: peer_id.clone().into(),
                peer_uri: Url::parse(&format!("mock://{}?{}", space_address, peer_id)).unwrap(),
                timestamp: 0,
            },
        }
    }
    fn this_peer(&self) -> &PeerData {
        &self.this_peer
    }
    fn as_dht_mut() -> DhtEndpointWithContext {

    }
}*/

#[allow(dead_code)]
pub struct GhostEngine<'engine> {
    /// Identifier
    name: String,
    /// Config settings
    config: RealEngineConfig,
    /// Factory for building DHT's of type D
    dht_factory: DhtFactory,
    /// Tracking request_id's sent to core
    request_track: Tracker<RealEngineTrackerData>,
    // TODO #176: Remove this if we resolve #176 without it.
    #[allow(dead_code)]
    /// Transport used by the network gateway
    network_transport: TransportWrapper<'engine>,
    /// P2p gateway for the network layer
    network_gateway: GatewayWrapper<'engine>,
    /// Store active connections?
    network_connections: HashSet<ConnectionId>,
    /// Map of P2p gateway per Space+Agent
    space_gateway_map: HashMap<ChainId, GatewayWrapper<'engine>>,
    #[allow(dead_code)]
    /// crypto system to use
    crypto: Box<dyn CryptoSystem>,
    #[allow(dead_code)]
    /// transport_id data, public/private keys, etc
    transport_keys: TransportKeys,
    /// debug: count number of calls to process()
    process_count: u64,

    client_endpoint: Option<
        GhostEndpoint<
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hToClient,
            Lib3hToClientResponse,
            Lib3hError,
        >,
    >,
    lib3h_endpoint: Detach<
        GhostContextEndpoint<
            GhostEngine<'engine>,
            RequestContext,
            Lib3hToClient,
            Lib3hToClientResponse,
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hError,
        >,
    >,
}

impl<'engine> GhostEngine<'engine> {
    pub fn new(
        name: &str,
        crypto: Box<dyn CryptoSystem>,
        config: RealEngineConfig,
        dht_factory: DhtFactory,
        network_transport: TransportWrapper<'engine>,
    ) -> Lib3hResult<Self> {
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
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        Ok(GhostEngine {
            crypto,
            config,
            name: name.to_string(),
            dht_factory,
            request_track: Tracker::new("real_engine_", 2000),
            network_transport,
            network_gateway,
            network_connections: HashSet::new(),
            space_gateway_map: HashMap::new(),
            transport_keys,
            process_count: 0,
            client_endpoint: Some(endpoint_parent),
            lib3h_endpoint: Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix(name)
                    .build(),
            ),
        })
    }
}

impl<'engine>
    GhostActor<
        Lib3hToClient,
        Lib3hToClientResponse,
        ClientToLib3h,
        ClientToLib3hResponse,
        Lib3hError,
    > for GhostEngine<'engine>
{
    // START BOILER PLATE--------------------------
    fn take_parent_endpoint(
        &mut self,
    ) -> Option<
        GhostEndpoint<
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hToClient,
            Lib3hToClientResponse,
            Lib3hError,
        >,
    > {
        std::mem::replace(&mut self.client_endpoint, None)
    }
    // END BOILER PLATE--------------------------

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // START BOILER PLATE--------------------------
        // always run the endpoint process loop
        detach_run!(&mut self.lib3h_endpoint, |cs| { cs.process(self) })?;
        // END BOILER PLATE--------------------------

        for msg in self.lib3h_endpoint.as_mut().drain_messages() {
            self.handle_msg_from_client(msg)?;
        }

        Ok(true.into())
    }
}

/// Drop
impl<'engine> Drop for GhostEngine<'engine> {
    fn drop(&mut self) {
        self.shutdown().unwrap_or_else(|e| {
            warn!("Graceful shutdown failed: {}", e);
        });
    }
}

/// Private
impl<'engine> GhostEngine<'engine> {
    /// Called on drop.
    /// Close all connections gracefully
    fn shutdown(&mut self) -> Lib3hResult<()> {
        Ok(())
        /*   let mut result: Lib3hResult<()> = Ok(());

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

        result*/
    }

    /// Process any Client events or requests
    fn handle_msg_from_client(&mut self, mut msg: ClientToLib3hMessage) -> Result<(), GhostError> {
        match msg.take_message().expect("exists") {
            ClientToLib3h::Connect(_data) => Ok(()),
            //    let cmd = TransportCommand::Connect(data.peer_uri, data.request_id);
            //  self.network_gateway.as_transport_mut().post(cmd)?;
            // ARG need this to be a request with a callback!!
            // msg.respond(Err(Lib3hError::new_other("connection failed!".into())));
            ClientToLib3h::JoinSpace(data) => {
                let result = self
                    .handle_join(&data)
                    .map(|_| ClientToLib3hResponse::JoinSpaceResult);
                msg.respond(result)
            }
            ClientToLib3h::LeaveSpace(data) => {
                let result = self
                    .handle_leave(&data)
                    .map(|_| ClientToLib3hResponse::LeaveSpaceResult);
                msg.respond(result)
            }
            ClientToLib3h::SendDirectMessage(data) => {
                let result = self
                    .handle_direct_message(&data, false)
                    .map(|data| ClientToLib3hResponse::SendDirectMessageResult(data));
                msg.respond(result)
            }
            /*            FetchEntry(FetchEntryData)  => {} Not being used, probably deprecated*/
            ClientToLib3h::PublishEntry(data) => self
                .handle_publish_entry(&data)
                .map_err(|e| GhostError::from(e.to_string())),
            ClientToLib3h::HoldEntry(data) => self
                .handle_hold_entry(&data)
                .map_err(|e| GhostError::from(e.to_string())),
            ClientToLib3h::QueryEntry(data) => {
                let _ = self.handle_query_entry(msg, &data);
                Ok(())
            }
            _ => panic!("{:?} not implemented", msg),
        }
    }

    /// create a new gateway and add it to our gateway map
    fn add_gateway(
        &mut self,
        space_address: Address,
        agent_id: Address,
    ) -> Lib3hResult<(Address, Address)> {
        let chain_id = (space_address.clone(), agent_id.clone());
        if self.space_gateway_map.contains_key(&chain_id) {
            return Err(Lib3hError::new_other("Already joined space"));
        }

        // First create DhtConfig for space gateway

        /*TODO: FIXME
        let this_peer_transport_id_as_uri = {
                let gateway = self.network_gateway.as_ref();
                // TODO #175 - encapsulate this conversion logic
                Url::parse(format!("transportId:{}", gateway.this_peer().peer_address).as_str())
                    .expect("can parse url")
        };
        */
        let this_peer_transport_id_as_uri = {
            Url::parse(
                format!(
                    "transportId:{}",
                    format!("transport_id_for_{}", agent_id.clone())
                )
                .as_str(),
            )
            .expect("can parse url")
        };
        let dht_config = DhtConfig {
            this_peer_address: agent_id.into(),
            this_peer_uri: this_peer_transport_id_as_uri,
            custom: self.config.dht_custom_config.clone(),
            gossip_interval: self.config.dht_gossip_interval,
            timeout_threshold: self.config.dht_timeout_threshold,
        };
        // Create new space gateway for this ChainId
        let new_space_gateway: GatewayWrapper<'engine> =
            GatewayWrapper::new(P2pGateway::new_with_space(
                &space_address,
                self.network_gateway.as_transport(),
                self.dht_factory,
                &dht_config,
            ));
        //        let new_space_gateway = MockGateway::new(chain_id.clone());
        self.space_gateway_map
            .insert(chain_id.clone(), new_space_gateway);
        Ok(chain_id)
    }

    fn broadcast_join(&mut self, space_address: Address, peer: PeerData) {
        // TODO #150 - Send JoinSpace to all known peers
        let mut payload = Vec::new();
        let p2p_msg = P2pProtocol::BroadcastJoinSpace(space_address.clone().into(), peer.clone());
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
    }

    #[allow(non_snake_case)]
    fn handle_HandleGetAuthoringEntryListResult(&mut self, msg: EntryListData) -> Lib3hResult<()> {
        let space_gateway = self.get_space(
            &msg.space_address.to_owned(),
            &msg.provider_agent_id.to_owned(),
        )?;

        for (entry_address, aspect_address_list) in msg.address_map.clone() {
            // Check aspects and only request entry with new aspects
            space_gateway.as_mut().as_dht_mut().request(
                DhtContext::RequestAspectsOf {
                    entry_address: entry_address.clone(),
                    aspect_address_list,
                    msg: msg.clone(),
                    request_id: self.request_track.reserve(),
                },
                DhtRequestToChild::RequestAspectsOf(entry_address.clone()),
                Box::new(|ud, context, response| {
                    let (entry_address, aspect_address_list, msg, request_id) = {
                        if let DhtContext::RequestAspectsOf {
                            entry_address,
                            aspect_address_list,
                            msg,
                            request_id,
                        } = context
                        {
                            (entry_address, aspect_address_list, msg, request_id)
                        } else {
                            panic!("bad context type");
                        }
                    };
                    let response = {
                        match response {
                            GhostCallbackData::Timeout => panic!("timeout"),
                            GhostCallbackData::Response(response) => match response {
                                Err(e) => panic!("{:?}", e),
                                Ok(response) => response,
                            },
                        }
                    };
                    if let DhtRequestToChildResponse::RequestAspectsOf(maybe_known_aspects) =
                        response
                    {
                        let can_fetch = match maybe_known_aspects {
                            None => true,
                            Some(known_aspects) => {
                                let can = !includes(&known_aspects, &aspect_address_list);
                                can
                            }
                        };
                        if can_fetch {
                            let msg_data = FetchEntryData {
                                space_address: msg.space_address.clone(),
                                entry_address: entry_address.clone(),
                                request_id,
                                provider_agent_id: msg.provider_agent_id.clone(),
                                aspect_address_list: None,
                            };

                            let context = RequestContext {
                                space_address: msg.space_address.to_owned(),
                                agent_id: msg.provider_agent_id.to_owned(),
                            };
                            let _ = self.lib3h_endpoint.request(
                                context.clone(),
                                Lib3hToClient::HandleFetchEntry(msg_data),
                                Box::new(|me, context, response| {
                                    let space_gateway = me
                                        .get_space(
                                            &context.space_address.to_owned(),
                                            &context.agent_id.to_owned(),
                                        )
                                        .map_err(|e| GhostError::from(e.to_string()))?;
                                    match response {
                                        GhostCallbackData::Response(Ok(
                                            Lib3hToClientResponse::HandleFetchEntryResult(msg),
                                        )) => {
                                            space_gateway.as_mut().as_dht_mut().publish(
                                                DhtRequestToChild::BroadcastEntry(
                                                    msg.entry.clone(),
                                                ),
                                            )?;
                                        }
                                        GhostCallbackData::Response(Err(e)) => {
                                            error!("Got error on HandleFetchEntryResult: {:?} ", e);
                                        }
                                        GhostCallbackData::Timeout => {
                                            error!("Got timeout on HandleFetchEntryResult");
                                        }
                                        _ => panic!("bad response type"),
                                    }
                                    Ok(())
                                }),
                            );
                            /*                            ud.lib3h_outbox
                            .push(Lib3hServerProtocol::HandleFetchEntry(msg_data));*/
                        }
                    } else {
                        panic!("bad response to RequestAspectsOf: {:?}", response);
                    }
                    Ok(())
                }),
            )?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    fn handle_HandleGetGossipingEntryListResult(&mut self, msg: EntryListData) -> Lib3hResult<()> {
        let space_gateway = self.get_space(
            &msg.space_address.to_owned(),
            &msg.provider_agent_id.to_owned(),
        )?;

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
                .as_mut()
                .as_dht_mut()
                .publish(DhtRequestToChild::HoldEntryAspectAddress(fake_entry))
                .map_err(|e| Lib3hError::new_other(&e.to_string()))?;
        }
        Ok(())
    }

    /// Create a gateway for this agent in this space, if not already part of it.
    /// Must not already be part of this space.
    fn handle_join(&mut self, join_msg: &SpaceData) -> Lib3hResult<()> {
        let chain_id =
            self.add_gateway(join_msg.space_address.clone(), join_msg.agent_id.clone())?;

        let this_peer = {
            let space_gateway = self.space_gateway_map.get_mut(&chain_id).unwrap();
            let this_peer = { space_gateway.as_mut().get_this_peer_sync().clone() };
            self.broadcast_join(join_msg.space_address.clone(), this_peer.clone());
            this_peer
        };

        let space_gateway = self.space_gateway_map.get_mut(&chain_id).unwrap();

        // Have DHT broadcast our PeerData
        space_gateway
            .as_mut()
            .as_dht_mut()
            .publish(DhtRequestToChild::HoldPeer(this_peer))?;

        // Send Get*Lists requests
        let mut list_data = GetListData {
            space_address: join_msg.space_address.clone(),
            provider_agent_id: join_msg.agent_id.clone(),
            request_id: self.request_track.reserve(),
        };
        let context = RequestContext {
            space_address: join_msg.space_address.to_owned(),
            agent_id: join_msg.agent_id.to_owned(),
        };
        self.lib3h_endpoint.request(
            context.clone(),
            Lib3hToClient::HandleGetGossipingEntryList(list_data.clone()),
            Box::new(|me, _ctx, response| {
                match response {
                    GhostCallbackData::Response(Ok(
                        Lib3hToClientResponse::HandleGetGossipingEntryListResult(msg),
                    )) => {
                        if let Err(err) = me.handle_HandleGetGossipingEntryListResult(msg) {
                            error!(
                                "Got error when handling HandleGetGossipingEntryListResult: {:?} ",
                                err
                            );
                        };
                    }
                    GhostCallbackData::Response(Err(e)) => {
                        error!("Got error from HandleGetGossipingEntryListResult: {:?} ", e);
                    }
                    GhostCallbackData::Timeout => {
                        error!("Got timeout on HandleGetGossipingEntryListResult");
                    }
                    _ => panic!("bad response type"),
                }
                Ok(())
            }),
        )?;

        list_data.request_id = self.request_track.reserve();
        self.lib3h_endpoint
            .request(
                context,
                Lib3hToClient::HandleGetAuthoringEntryList(list_data.clone()),
                Box::new(|me, _ctx, response| {
                    match response {
                        GhostCallbackData::Response(Ok(
                            Lib3hToClientResponse::HandleGetAuthoringEntryListResult(msg),
                        )) => {
                            if let Err(err) = me.handle_HandleGetAuthoringEntryListResult(msg) {
                                error!(
                                "Got error when handling HandleGetAuthoringEntryListResult: {:?} ",
                                err
                            );
                            };
                        }
                        GhostCallbackData::Response(Err(e)) => {
                            error!("Got error on HandleGetAuthoringEntryListResult: {:?} ", e);
                        }
                        GhostCallbackData::Timeout => {
                            error!("Got timeout on HandleGetAuthoringEntryListResult");
                        }
                        _ => panic!("bad response type"),
                    }
                    Ok(())
                }),
            )
            .map_err(|e| Lib3hError::new(ErrorKind::Other(e.to_string())))
        // Done
        //Ok(())
    }

    /// Destroy gateway for this agent in this space, if part of it.
    fn handle_leave(&mut self, join_msg: &SpaceData) -> Lib3hResult<()> {
        let chain_id = (join_msg.space_address.clone(), join_msg.agent_id.clone());
        match self.space_gateway_map.remove(&chain_id) {
            Some(_) => Ok(()), //TODO is there shutdown code we need to call
            None => Err(Lib3hError::new_other("Not part of that space")),
        }
    }

    fn handle_direct_message(
        &mut self,
        msg: &DirectMessageData,
        is_response: bool,
    ) -> Lib3hResult<DirectMessageData> {
        let chain_id = (msg.space_address.clone(), msg.from_agent_id.clone());
        let space_gateway = self
            .space_gateway_map
            .get_mut(&chain_id)
            .ok_or(Lib3hError::new_other("Not part of that space"))?;

        // Check if messaging self
        let peer_address = { space_gateway.as_mut().get_this_peer_sync().peer_address };
        let to_agent_id: String = msg.to_agent_id.clone().into();
        if &peer_address == &to_agent_id {
            return Err(Lib3hError::new_other("messaging self not allowed"));
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
        let _peer_address: String = msg.to_agent_id.clone().into();
        /* TODO: fix when gateway implemented
            let res = space_gateway
                .as_transport_mut()
                .send(&[peer_address.as_str()], &payload);
            if let Err(e) = res {
                response.result_info = e.to_string().as_bytes().to_vec();
                return Lib3hServerProtocol::FailureResult(response);
        }*/
        // TODO: FAKE MESSAGE
        Ok(DirectMessageData {
            space_address: msg.space_address.clone(),
            request_id: msg.request_id.clone(),
            to_agent_id: msg.from_agent_id.clone(),
            from_agent_id: msg.to_agent_id.clone(),
            content: b"fake response".to_vec(),
        })
    }

    fn handle_publish_entry(&mut self, msg: &ProvidedEntryData) -> Lib3hResult<()> {
        let space_gateway = self.get_space(
            &msg.space_address.to_owned(),
            &msg.provider_agent_id.to_owned(),
        )?;
        space_gateway
            .as_mut()
            .as_dht_mut()
            .publish(DhtRequestToChild::BroadcastEntry(msg.entry.clone()))
            .map_err(|e| Lib3hError::new_other(&e.to_string()))
    }

    fn handle_hold_entry(&mut self, msg: &ProvidedEntryData) -> Lib3hResult<()> {
        let space_gateway = self.get_space(
            &msg.space_address.to_owned(),
            &msg.provider_agent_id.to_owned(),
        )?;
        space_gateway
            .as_mut()
            .as_dht_mut()
            .publish(DhtRequestToChild::HoldEntryAspectAddress(msg.entry.clone()))
            .map_err(|e| Lib3hError::new_other(&e.to_string()))
    }

    fn handle_query_entry(
        &mut self,
        msg: ClientToLib3hMessage,
        data: &QueryEntryData,
    ) -> Lib3hResult<()> {
        let chain_id = (data.space_address.clone(), data.requester_agent_id.clone());
        let _space_gateway = self
            .space_gateway_map
            .get_mut(&chain_id)
            .ok_or(Lib3hError::new_other("Not part of that space"))?;
        /*
                let context = "".to_string();
                    //DhtContext::RequestEntry { }
                space_gateway
                    .as_mut()
                    .as_dht_mut()
                    .request(
                        context,
                        DhtRequestToChild::RequestEntry(data.entry_address),
                        Box::new(|_me, _context, response| {
                            match response {
                                GhostCallbackData::Response(Ok(
                                    DhtRequestToChildResponse::RequestEntry(entry_data),
                                )) => {

                                }
                                GhostCallbackData::Response(Err(e)) => {
                                    error!("Got error on DHT RequestEntry: {:?} ", e);
                                }
                                GhostCallbackData::Timeout => {
                                    error!("Got timeout on DHT RequestEntry");
                                }
                                _ => panic!("bad response type"),
                            }
                            Ok(())
                        }
                    ))?;
        */
        // FAKE
        let result = Ok(QueryEntryResultData {
            space_address: data.space_address.clone(),
            entry_address: data.entry_address.clone(),
            request_id: data.request_id.clone(),
            requester_agent_id: data.requester_agent_id.clone(),
            responder_agent_id: "fake_responder_id".into(),
            query_result: b"fake response".to_vec(),
        })
        .map(|data| ClientToLib3hResponse::QueryEntryResult(data));

        msg.respond(result)
            .map_err(|e| Lib3hError::new_other(&e.to_string()))
    }

    /// Get a space_gateway for the specified space+agent.
    /// If agent did not join that space, construct error
    fn get_space(
        &mut self,
        space_address: &Address,
        agent_id: &Address,
    ) -> Lib3hResult<&mut GatewayWrapper<'engine>> {
        self.space_gateway_map
            .get_mut(&(space_address.to_owned(), agent_id.to_owned()))
            .ok_or(Lib3hError::new_other(&format!(
                "Not in space: {:?},{:?}",
                space_address, agent_id
            )))
    }
}

/// Return true if all elements of list_b are found in list_a
fn includes(list_a: &[Address], list_b: &[Address]) -> bool {
    let set_a: HashSet<_> = list_a.iter().map(|addr| addr).collect();
    let set_b: HashSet<_> = list_b.iter().map(|addr| addr).collect();
    set_b.is_subset(&set_a)
}

#[cfg(test)]
mod tests {
    use super::*;
    //    use lib3h_protocol::data_types::*;
    struct MockCore {
        //    state: String,
    }
    use crate::{
        dht::mirror_dht::MirrorDht, transport::memory_mock::transport_memory::TransportMemory,
        transport_wss::TlsConfig,
    };
    use url::Url;

    use lib3h_sodium::SodiumCryptoSystem;

    fn make_test_entry() -> EntryData {
        let aspect_list = Vec::new();
        EntryData {
            entry_address: "fake_address".into(),
            aspect_list,
        }
    }

    #[test]
    fn test_ghost_engine() {
        let mut _core = MockCore {
            //        state: "".to_string(),
        };

        let network_transport = TransportWrapper::new(TransportMemory::new());
        let crypto = Box::new(SodiumCryptoSystem::new());
        let config = RealEngineConfig {
            tls_config: TlsConfig::Unencrypted,
            socket_type: "mem".into(),
            bootstrap_nodes: vec![],
            work_dir: String::new(),
            log_level: 'd',
            bind_url: Url::parse(format!("mem://{}", "test_engine").as_str()).unwrap(),
            dht_gossip_interval: 100,
            dht_timeout_threshold: 1000,
            dht_custom_config: vec![],
        };
        let dht_factory = MirrorDht::new_with_config;

        let engine: GhostEngine = GhostEngine::new(
            "test_engine",
            crypto,
            config,
            dht_factory,
            network_transport,
        )
        .unwrap();
        let mut lib3h: GhostEngineParentWrapper<MockCore, RequestContext, GhostEngine, Lib3hError> =
            GhostParentWrapper::new(engine, "test_engine");
        assert_eq!(lib3h.as_ref().space_gateway_map.len(), 0);

        let req_data = SpaceData {
            /// Identifier of this request
            request_id: "foo_id".into(),
            space_address: "space_addr".into(),
            agent_id: "agent_id".into(),
        };

        let result = lib3h.as_mut().handle_join(&req_data);
        assert!(result.is_ok());
        assert_eq!(lib3h.as_ref().space_gateway_map.len(), 1);
        let result = lib3h.as_mut().handle_join(&req_data);
        assert_eq!(
            "Err(Lib3hError(Other(\"Already joined space\")))",
            format!("{:?}", result)
        );

        let direct_message = DirectMessageData {
            request_id: "foo_id".into(),
            space_address: "space_addr".into(),
            from_agent_id: "agent_id".into(),
            to_agent_id: "to_agent_id".into(),
            content: b"foo content".to_vec(),
        };

        let result = lib3h.as_mut().handle_direct_message(&direct_message, false);
        // TODO: clean up test when possbie: this is fake data because we don't really have a gateway, bu
        assert_eq!(
            "Ok(DirectMessageData { space_address: HashString(\"space_addr\"), request_id: \"foo_id\", to_agent_id: HashString(\"agent_id\"), from_agent_id: HashString(\"to_agent_id\"), content: [102, 97, 107, 101, 32, 114, 101, 115, 112, 111, 110, 115, 101] })",
            format!("{:?}", result)
        );

        let entry_data = ProvidedEntryData {
            space_address: "space_addr".into(),
            provider_agent_id: "agent_id".into(),
            entry: make_test_entry(),
        };

        let result = lib3h.as_mut().handle_publish_entry(&entry_data);
        assert!(result.is_ok());

        let result = lib3h.as_mut().handle_hold_entry(&entry_data);
        assert!(result.is_ok());

        let result = lib3h.as_mut().handle_leave(&req_data);
        assert!(result.is_ok());

        let result = lib3h.as_mut().handle_leave(&req_data);
        assert_eq!(
            "Err(Lib3hError(Other(\"Not part of that space\")))",
            format!("{:?}", result)
        );
    }
}
