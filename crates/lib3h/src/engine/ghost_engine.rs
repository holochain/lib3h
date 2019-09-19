use detach::Detach;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::*, protocol::*, Address};
use std::collections::{HashMap, HashSet};

use crate::{
    dht::{dht_config::DhtConfig, dht_protocol::*},
    engine::{
        engine_actor::*, p2p_protocol::*, CanAdvertise, ChainId, EngineConfig, GatewayId,
        GhostEngine, TransportConfig, TransportKeys,
    },
    error::{ErrorKind, Lib3hError, Lib3hResult},
    gateway::{protocol::*, P2pGateway},
    keystore::KeystoreStub,
    track::Tracker,
    transport::{
        self, memory_mock::ghost_transport_memory::*, protocol::*,
        websocket::actor::GhostTransportWebsocket, TransportEncoding, TransportMultiplex,
    },
};
use holochain_tracing::Span;
use lib3h_crypto_api::CryptoSystem;
use rmp_serde::Serializer;
use serde::Serialize;
use url::Url;

impl<'engine> CanAdvertise for GhostEngine<'engine> {
    fn advertise(&self) -> Url {
        self.this_net_peer.peer_uri.to_owned()
    }
}
impl<'engine> GhostEngine<'engine> {
    /// Constructor with for GhostEngine
    pub fn new(
        span: Span,
        crypto: Box<dyn CryptoSystem>,
        config: EngineConfig,
        name: &str,
        dht_factory: DhtFactory,
    ) -> Lib3hResult<Self> {
        let transport_keys = TransportKeys::new(crypto.as_crypto_system())?;

        // This will change when multi-transport is impelmented
        assert_eq!(config.transport_configs.len(), 1);
        let transport_config = config.transport_configs[0].clone();
        let machine_id = transport_keys.transport_id.clone().into();
        let transport: DynTransportActor = match &transport_config {
            TransportConfig::Websocket(tls_config) => {
                let tls = tls_config.clone();
                Box::new(GhostTransportWebsocket::new(machine_id, tls))
            }
            TransportConfig::Memory(net) => Box::new(GhostTransportMemory::new(machine_id, &net)),
        };

        let transport = TransportEncoding::new(
            crypto.box_clone(),
            transport_keys.transport_id.clone(),
            "NETWORK_ID_STUB".to_string(),
            Box::new(KeystoreStub::new()),
            transport,
        );

        let prebound_binding = Url::parse("none:").unwrap();
        let this_net_peer = PeerData {
            peer_address: format!("{}_tId", name),
            peer_uri: prebound_binding,
            timestamp: 0, // TODO #166
        };
        // Create DhtConfig
        let dht_config = DhtConfig::with_engine_config(&format!("{}_tId", name), &config);
        debug!("New MOCK Engine {} -> {:?}", name, this_net_peer);
        let mut multiplexer = Detach::new(GatewayParentWrapper::new(
            TransportMultiplex::new(P2pGateway::new(
                config.network_id.clone(),
                Box::new(transport),
                dht_factory,
                &dht_config,
            )),
            "to_multiplexer_",
        ));

        // Bind & create this_net_peer
        // TODO: Find better way to do init with GhostEngine
        multiplexer.as_mut().request(
            Span::fixme(),
            GatewayRequestToChild::Transport(RequestToChild::Bind {
                spec: config.bind_url.clone(),
            }),
            Box::new(|me: &mut GhostEngine<'engine>, response| {
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => match response {
                            Err(e) => panic!("{:?}", e),
                            Ok(response) => response,
                        },
                    }
                };
                if let GatewayRequestToChildResponse::Transport(RequestToChildResponse::Bind(
                    bind_data,
                )) = response
                {
                    me.this_net_peer.peer_uri = bind_data.bound_url;
                } else {
                    panic!("bad response to bind: {:?}", response);
                }
                Ok(())
            }),
        )?;

        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let mut engine = GhostEngine {
            crypto,
            config,
            name: name.to_string(),
            dht_factory,
            request_track: Tracker::new("real_engine_", 2000),
            multiplexer,
            this_net_peer,
            network_connections: HashSet::new(),
            space_gateway_map: HashMap::new(),
            transport_keys,
            client_endpoint: Some(endpoint_parent),
            lib3h_endpoint: Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix(name)
                    .build(),
            ),
        };
        detach_run!(engine.multiplexer, |e| e.process(&mut engine))?;
        engine.multiplexer.as_mut().publish(
            Span::fixme(),
            GatewayRequestToChild::Dht(DhtRequestToChild::UpdateAdvertise(
                engine.this_net_peer.peer_uri.clone(),
            )),
        )?;
        detach_run!(engine.multiplexer, |e| e.process(&mut engine))?;
        engine.priv_connect_bootstraps(span)?;
        Ok(engine)
    }

    fn priv_connect_bootstraps(&mut self, span: Span) -> GhostResult<()> {
        let nodes: Vec<Url> = self.config.bootstrap_nodes.drain(..).collect();
        for bs in nodes {
            // can't use handle_bootstrap() because it assumes a message to respond to
            let cmd = GatewayRequestToChild::Bootstrap(BootstrapData {
                space_address: self.config.network_id.id.clone(),
                bootstrap_uri: bs,
            });
            self.multiplexer.request(
                span.child("priv_connect_bootstrap TODO extra info"),
                cmd,
                Box::new(|_, response| {
                    let response = match response {
                        GhostCallbackData::Timeout => panic!("bootstrap timeout"),
                        GhostCallbackData::Response(r) => r,
                    };
                    if let Err(e) = response {
                        panic!("{:?}", e);
                    }
                    Ok(())
                }),
            )?;
        }
        Ok(())
    }

    pub fn this_space_peer(&mut self, chain_id: ChainId) -> Lib3hResult<PeerData> {
        trace!("engine.this_space_peer() ...");
        let space_gateway = self
            .space_gateway_map
            .get_mut(&chain_id)
            .ok_or_else(|| Lib3hError::from("No space at chainId"))?;
        Ok(space_gateway.as_mut().as_mut().this_peer())
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
        let /*mut*/ result: Lib3hResult<()> = Ok(());

        // TODO: #328
        //        for space_gatway in self.space_gateway_map.values_mut() {
        //            let res = space_gatway.as_transport_mut().close_all();
        //            // Continue closing connections even if some failed
        //            if let Err(e) = res {
        //                if result.is_ok() {
        //                    result = Err(e.into());
        //                }
        //            }
        //        }
        //        self.multiplexer
        //            .as_transport_mut()
        //            .close_all()
        //            .map_err(|e| {
        //                error!("Closing of some connection failed: {:?}", e);
        //                e
        //            })?;

        result
    }

    /// Process connect events by sending them to the multiplexer
    fn handle_bootstrap(
        &mut self,
        msg: ClientToLib3hMessage,
        data: BootstrapData,
    ) -> GhostResult<()> {
        self.multiplexer.request(
            Span::fixme(),
            GatewayRequestToChild::Bootstrap(data),
            Box::new(move |_me, response| {
                match response {
                    GhostCallbackData::Response(Ok(
                        GatewayRequestToChildResponse::BootstrapSuccess,
                    )) => msg.respond(Ok(ClientToLib3hResponse::BootstrapSuccess))?,
                    GhostCallbackData::Response(Err(e)) => msg.respond(Err(e))?,
                    GhostCallbackData::Timeout => msg.respond(Err("timeout".into()))?,
                    _ => msg.respond(Err(format!("bad response: {:?}", response).into()))?,
                }
                Ok(())
            }),
        )
    }

    /// Process any Client events or requests
    pub(crate) fn handle_msg_from_client(
        &mut self,
        mut msg: ClientToLib3hMessage,
    ) -> GhostResult<()> {
        let span = msg.span().child("handle_msg_from_client");
        match msg.take_message().expect("exists") {
            ClientToLib3h::Bootstrap(data) => {
                trace!("ClientToLib3h::Bootstrap: {:?}", &data);
                self.handle_bootstrap(msg, data)
            }
            ClientToLib3h::JoinSpace(data) => {
                trace!("ClientToLib3h::JoinSpace: {:?}", data);
                let result = self
                    .handle_join(span.follower("TODO name"), &data)
                    .map(|_| ClientToLib3hResponse::JoinSpaceResult);
                msg.respond(result)
            }
            ClientToLib3h::LeaveSpace(data) => {
                trace!("ClientToLib3h::LeaveSpace: {:?}", data);
                let result = self
                    .handle_leave(span.follower("TODO name"), &data)
                    .map(|_| ClientToLib3hResponse::LeaveSpaceResult);
                msg.respond(result)
            }
            ClientToLib3h::SendDirectMessage(data) => {
                trace!("ClientToLib3h::SendDirectMessage: {:?}", data);
                self.handle_direct_message(span.follower("TODO name"), msg, &data)
                    .map_err(|e| GhostError::from(e.to_string()))
            }
            ClientToLib3h::PublishEntry(data) => {
                trace!("ClientToLib3h::PublishEntry: {:?}", data);
                self.handle_publish_entry(span.follower("TODO name"), &data)
                    .map_err(|e| GhostError::from(e.to_string()))
            }
            ClientToLib3h::HoldEntry(data) => {
                trace!("ClientToLib3h::HoldEntry: {:?}", data);
                self.handle_hold_entry(span.follower("TODO name"), &data)
                    .map_err(|e| GhostError::from(e.to_string()))
            }
            ClientToLib3h::QueryEntry(data) => {
                trace!("ClientToLib3h::QueryEntry: {:?}", data);
                self.handle_query_entry(span.follower("TODO name"), msg, data)
                    .map_err(|e| GhostError::from(e.to_string()))
            }
            ClientToLib3h::FetchEntry(_) => panic!("FetchEntry Deprecated"),
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

        let dht_config = DhtConfig::with_engine_config(&agent_id.to_string(), &self.config);

        // Create new space gateway for this ChainId
        let uniplex = TransportEndpointAsActor::new(
            self.multiplexer
                .as_mut()
                .as_mut()
                .create_agent_space_route(&space_address, &agent_id),
        );
        let uniplex = TransportEncoding::new(
            self.crypto.box_clone(),
            agent_id.to_string(),
            space_address.clone().into(),
            Box::new(KeystoreStub::new()),
            Box::new(uniplex),
        );

        let gateway_id = GatewayId {
            id: space_address.clone(),
            nickname: format!(
                "{}_{}",
                space_address.to_string().split_at(4).0,
                agent_id.to_string().split_at(4).0
            ),
        };
        let new_space_gateway = Detach::new(GatewayParentWrapper::new(
            P2pGateway::new(gateway_id, Box::new(uniplex), self.dht_factory, &dht_config),
            "space_gateway_",
        ));
        self.space_gateway_map
            .insert(chain_id.clone(), new_space_gateway);
        Ok(chain_id)
    }

    fn broadcast_join(
        &mut self,
        span: Span,
        space_address: Address,
        peer: PeerData,
    ) -> GhostResult<()> {
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
        self.multiplexer
            .publish(span, GatewayRequestToChild::SendAll(payload))
        // TODO END
    }

    #[allow(non_snake_case)]
    fn handle_HandleGetAuthoringEntryListResult(
        &mut self,
        entry_list: EntryListData,
    ) -> Lib3hResult<()> {
        let space_gateway = self.get_space(
            &entry_list.space_address.to_owned(),
            &entry_list.provider_agent_id.to_owned(),
        )?;

        /*let x =                   DhtContext::RequestAspectsOf {
            entry_address: entry_address.clone(),
            aspect_address_list,
            msg: msg.clone(),
            request_id: self.request_track.reserve(),
        };*/

        for (entry_address, aspect_address_list) in entry_list.address_map.clone() {
            let space_address = entry_list.space_address.clone();
            let provider_agent_id = entry_list.provider_agent_id.clone();
            // Check aspects and only request entry with new aspects
            space_gateway.request(
                Span::fixme(),
                GatewayRequestToChild::Dht(DhtRequestToChild::RequestAspectsOf(
                    entry_address.clone(),
                )),
                Box::new(move |me, response| {
                    let response = {
                        match response {
                            GhostCallbackData::Timeout => return Err("timeout".into()),
                            GhostCallbackData::Response(response) => match response {
                                Err(e) => return Err(e.into()),
                                Ok(response) => response,
                            },
                        }
                    };
                    if let GatewayRequestToChildResponse::Dht(
                        DhtRequestToChildResponse::RequestAspectsOf(maybe_known_aspects),
                    ) = response
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
                                space_address: space_address.clone(),
                                entry_address: entry_address.clone(),
                                request_id: me.request_track.reserve(),
                                provider_agent_id: provider_agent_id.clone(),
                                aspect_address_list: None,
                            };

                            me.lib3h_endpoint.request(
                                Span::fixme(),
                                Lib3hToClient::HandleFetchEntry(msg_data),
                                Box::new(move |me, response| {
                                    let space_gateway = me
                                        .get_space(
                                            &space_address.to_owned(),
                                            &provider_agent_id.to_owned(),
                                        )
                                        .map_err(|e| GhostError::from(e.to_string()))?;
                                    match response {
                                        GhostCallbackData::Response(Ok(
                                            Lib3hToClientResponse::HandleFetchEntryResult(msg),
                                        )) => space_gateway.publish(
                                            Span::fixme(),
                                            GatewayRequestToChild::Dht(
                                                DhtRequestToChild::BroadcastEntry(
                                                    msg.entry.clone(),
                                                ),
                                            ),
                                        ),
                                        GhostCallbackData::Response(Err(e)) => Err(e.into()),
                                        GhostCallbackData::Timeout => Err("timeout".into()),
                                        _ => Err("bad response type".into()),
                                    }
                                }),
                            )?;
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
                    aspect: vec![].into(),
                    publish_ts: 0,
                };
                aspect_list.push(fake_aspect);
            }
            // Create "shallow" entry, in the sense an entry with no actual aspect content,
            // but valid addresses.
            let shallow_entry = EntryData {
                entry_address: entry_address.clone(),
                aspect_list,
            };
            space_gateway
                .publish(
                    Span::fixme(),
                    GatewayRequestToChild::Dht(DhtRequestToChild::HoldEntryAspectAddress(
                        shallow_entry,
                    )),
                )
                .map_err(|e| Lib3hError::new_other(&e.to_string()))?;
        }
        Ok(())
    }

    /// Create a gateway for this agent in this space, if not already part of it.
    /// Must not already be part of this space.
    fn handle_join(&mut self, span: Span, join_msg: &SpaceData) -> Lib3hResult<()> {
        let chain_id =
            self.add_gateway(join_msg.space_address.clone(), join_msg.agent_id.clone())?;

        let this_peer = self.this_space_peer(chain_id.clone())?;
        self.broadcast_join(
            span.child("broadcast_join"),
            join_msg.space_address.clone(),
            this_peer.clone(),
        )?;

        let space_gateway = self.space_gateway_map.get_mut(&chain_id).unwrap();

        // Have DHT broadcast our PeerData
        space_gateway.publish(
            span.follower("space_gateway.publish"),
            GatewayRequestToChild::Dht(DhtRequestToChild::HoldPeer(this_peer)),
        )?;

        // Send Get*Lists requests
        let mut list_data = GetListData {
            space_address: join_msg.space_address.clone(),
            provider_agent_id: join_msg.agent_id.clone(),
            request_id: self.request_track.reserve(),
        };

        self.lib3h_endpoint.request(
            span.follower("TODO"),
            Lib3hToClient::HandleGetGossipingEntryList(list_data.clone()),
            Box::new(|me, response| match response {
                GhostCallbackData::Response(Ok(
                    Lib3hToClientResponse::HandleGetGossipingEntryListResult(msg),
                )) => Ok(me.handle_HandleGetGossipingEntryListResult(msg)?),
                GhostCallbackData::Response(Err(e)) => Err(e.into()),
                GhostCallbackData::Timeout => Err("timeout".into()),
                _ => Err("bad response type".into()),
            }),
        )?;

        list_data.request_id = self.request_track.reserve();
        self.lib3h_endpoint
            .request(
                span.follower("TODO"),
                Lib3hToClient::HandleGetAuthoringEntryList(list_data.clone()),
                Box::new(|me, response| match response {
                    GhostCallbackData::Response(Ok(
                        Lib3hToClientResponse::HandleGetAuthoringEntryListResult(msg),
                    )) => Ok(me.handle_HandleGetAuthoringEntryListResult(msg)?),
                    GhostCallbackData::Response(Err(e)) => Err(e.into()),
                    GhostCallbackData::Timeout => Err("timeout".into()),
                    _ => Err("bad response type".into()),
                }),
            )
            .map_err(|e| Lib3hError::new(ErrorKind::Other(e.to_string())))
    }

    /// Destroy gateway for this agent in this space, if part of it.
    fn handle_leave(&mut self, _span: Span, join_msg: &SpaceData) -> Lib3hResult<()> {
        let chain_id = (join_msg.space_address.clone(), join_msg.agent_id.clone());
        match self.space_gateway_map.remove(&chain_id) {
            Some(_) => Ok(()), //TODO is there shutdown code we need to call
            None => Err(Lib3hError::new_other("Not part of that space")),
        }
    }

    fn handle_direct_message(
        &mut self,
        span: Span,
        ghost_message: ClientToLib3hMessage,
        msg: &DirectMessageData,
    ) -> Lib3hResult<()> {
        let chain_id = (msg.space_address.clone(), msg.from_agent_id.clone());

        let maybe_this_peer = self.this_space_peer(chain_id.clone());
        if let Err(error) = maybe_this_peer {
            ghost_message.respond(Err(error))?;
            return Ok(());
        };
        let this_peer = maybe_this_peer.unwrap();

        let to_agent_id: String = msg.to_agent_id.clone().into();
        if &this_peer.peer_address == &to_agent_id {
            return Err(Lib3hError::new_other("messaging self not allowed"));
        }

        let net_msg = P2pProtocol::DirectMessage(msg.clone());

        // Serialize payload
        let mut payload = Vec::new();
        net_msg
            .serialize(&mut Serializer::new(&mut payload))
            .unwrap();
        // Send
        let peer_address: String = msg.to_agent_id.clone().into();

        let space_gateway = self
            .space_gateway_map
            .get_mut(&chain_id)
            .ok_or_else(|| Lib3hError::new_other("Not part of that space"))?;

        space_gateway
            .request(
                span,
                GatewayRequestToChild::Transport(
                    transport::protocol::RequestToChild::SendMessage {
                        uri: Url::parse(&("agentId:".to_string() + &peer_address))
                            .expect("invalid url format"),
                        payload: payload.into(),
                    },
                ),
                Box::new(|_me, response| {
                    debug!("GhostEngine: response to handle_direct_message message: {:?}", response);
                    match response {
                        GhostCallbackData::Response(Ok(
                                GatewayRequestToChildResponse::Transport(
                                    transport::protocol::RequestToChildResponse::SendMessageSuccess
                        ))) => {
                            panic!("We Need to bookmark this ghost_message so that we can invoke it with a message from the remote peer when they send it back");
                        }
                        _ => ghost_message.respond(Err(format!("{:?}", response).into()))?,
                    };
                    Ok(())
                })
            )?;
        Ok(())
    }

    fn handle_publish_entry(&mut self, span: Span, msg: &ProvidedEntryData) -> Lib3hResult<()> {
        // MIRROR - reflecting hold for now
        for aspect in &msg.entry.aspect_list {
            let data = StoreEntryAspectData {
                request_id: self.request_track.reserve(),
                space_address: msg.space_address.clone(),
                provider_agent_id: msg.provider_agent_id.clone(),
                entry_address: msg.entry.entry_address.clone(),
                entry_aspect: aspect.clone(),
            };

            self.lib3h_endpoint.request(
                span.child("TODO add tags"),
                Lib3hToClient::HandleStoreEntryAspect(data),
                Box::new(move |_me, response| {
                    // should just be OK
                    debug!(
                        "On HandleStoreEntryAspect request from handle_publish_entry got: {:?} ",
                        response
                    );
                    Ok(())
                }),
            )?;
        }
        let space_gateway = self.get_space(
            &msg.space_address.to_owned(),
            &msg.provider_agent_id.to_owned(),
        )?;
        space_gateway
            .publish(
                span,
                GatewayRequestToChild::Dht(DhtRequestToChild::BroadcastEntry(msg.entry.clone())),
            )
            .map_err(|e| Lib3hError::new_other(&e.to_string()))
    }

    fn handle_hold_entry(&mut self, span: Span, msg: &ProvidedEntryData) -> Lib3hResult<()> {
        let space_gateway = self.get_space(
            &msg.space_address.to_owned(),
            &msg.provider_agent_id.to_owned(),
        )?;
        space_gateway
            .publish(
                span,
                GatewayRequestToChild::Dht(DhtRequestToChild::HoldEntryAspectAddress(
                    msg.entry.clone(),
                )),
            )
            .map_err(|e| Lib3hError::new_other(&e.to_string()))
    }

    fn handle_query_entry(
        &mut self,
        span: Span,
        msg: ClientToLib3hMessage,
        data: QueryEntryData,
    ) -> Lib3hResult<()> {
        // TODO #169 reflecting for now...
        // ultimately this should get forwarded to the
        // correct neighborhood
        self.lib3h_endpoint
            .request(
                span,
                Lib3hToClient::HandleQueryEntry(data.clone()),
                Box::new(move |_me, response| {
                    match response {
                        GhostCallbackData::Response(Ok(
                            Lib3hToClientResponse::HandleQueryEntryResult(data),
                        )) => msg.respond(Ok(ClientToLib3hResponse::QueryEntryResult(data)))?,
                        GhostCallbackData::Response(Err(e)) => {
                            error!("Got error on HandleQueryEntryResult: {:?} ", e);
                        }
                        GhostCallbackData::Timeout => {
                            error!("Got timeout on HandleQueryEntryResult");
                        }
                        _ => panic!("bad response type"),
                    }
                    Ok(())
                }),
            )
            .map_err(|e| Lib3hError::new_other(&e.to_string()))
    }

    /// Get a space_gateway for the specified space+agent.
    /// If agent did not join that space, construct error
    pub fn get_space(
        &mut self,
        space_address: &Address,
        agent_id: &Address,
    ) -> Lib3hResult<&mut Detach<GatewayParentWrapper<GhostEngine<'engine>, P2pGateway>>> {
        self.space_gateway_map
            .get_mut(&(space_address.to_owned(), agent_id.to_owned()))
            .ok_or_else(|| {
                Lib3hError::new_other(&format!("Not in space: {:?},{:?}", space_address, agent_id))
            })
    }
}

/// Return true if all elements of list_b are found in list_a
#[allow(dead_code)]
fn includes(list_a: &[Address], list_b: &[Address]) -> bool {
    let set_a: HashSet<_> = list_a.iter().map(|addr| addr).collect();
    let set_b: HashSet<_> = list_b.iter().map(|addr| addr).collect();
    set_b.is_subset(&set_a)
}

pub fn handle_gossip_to<
    G: GhostActor<
        GatewayRequestToParent,
        GatewayRequestToParentResponse,
        GatewayRequestToChild,
        GatewayRequestToChildResponse,
        Lib3hError,
    >,
>(
    gateway_identifier: Address,
    gateway: &mut GatewayParentWrapper<GhostEngine, G>,
    gossip_data: GossipToData,
) -> Lib3hResult<()> {
    debug!(
        "({}) handle_gossip_to: {:?}",
        gateway_identifier, gossip_data,
    );

    for to_peer_address in gossip_data.peer_address_list {
        // FIXME
        //            // TODO #150 - should not gossip to self in the first place
        //            let me = self.get_this_peer_sync(&mut gateway).peer_address;
        //            if to_peer_address == me {
        //                continue;
        //            }
        //            // TODO END

        // Convert DHT Gossip to P2P Gossip
        let p2p_gossip = P2pProtocol::Gossip(GossipData {
            space_address: gateway_identifier.clone(),
            to_peer_address: to_peer_address.clone().into(),
            from_peer_address: "FIXME".into(), // FIXME
            bundle: gossip_data.bundle.clone(),
        });
        let mut payload = Vec::new();
        p2p_gossip
            .serialize(&mut Serializer::new(&mut payload))
            .expect("P2pProtocol::Gossip serialization failed");
        // Forward gossip to the inner_transport
        // FIXME peer_address to Url convert
        let msg = transport::protocol::RequestToChild::SendMessage {
            uri: Url::parse(&("agentId:".to_string() + &to_peer_address)).expect("invalid Url"),
            payload: payload.into(),
        };
        gateway.publish(Span::fixme(), GatewayRequestToChild::Transport(msg))?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dht::mirror_dht::MirrorDht, engine::GatewayId, tests::enable_logging_for_test};
    use holochain_tracing::test_span;
    use lib3h_sodium::SodiumCryptoSystem;
    use std::path::PathBuf;
    use url::Url;

    struct MockCore {
        //    state: String,
    }

    // Real test network-id should be a hc version of sha256 of a string
    fn test_network_id() -> GatewayId {
        GatewayId {
            nickname: "unit-test-test-net".into(),
            id: "Hc_fake_addr_for_test-net".into(),
        }
    }

    fn make_test_engine(test_net: &str) -> GhostEngine<'static> {
        let crypto = Box::new(SodiumCryptoSystem::new());
        let config = EngineConfig {
            network_id: test_network_id(),
            transport_configs: vec![TransportConfig::Memory(test_net.into())],
            bootstrap_nodes: vec![],
            work_dir: PathBuf::new(),
            log_level: 'd',
            bind_url: Url::parse(format!("mem://{}", "test_engine").as_str()).unwrap(),
            dht_gossip_interval: 100,
            dht_timeout_threshold: 1000,
            dht_custom_config: vec![],
        };
        let dht_factory = MirrorDht::new_with_config;

        let engine =
            GhostEngine::new(test_span(""), crypto, config, "test_engine", dht_factory).unwrap();
        engine
    }

    fn make_test_engine_wrapper(
        net: &str,
    ) -> GhostEngineParentWrapper<MockCore, GhostEngine<'static>, Lib3hError> {
        let engine = make_test_engine(net);
        let lib3h: GhostEngineParentWrapper<MockCore, GhostEngine, Lib3hError> =
            GhostParentWrapper::new(engine, "test_engine");
        lib3h
    }

    #[test]
    fn test_ghost_engine_construct() {
        let lib3h = make_test_engine_wrapper("test_ghost_engine_construct");
        assert_eq!(lib3h.as_ref().space_gateway_map.len(), 0);

        // check that bootstrap nodes were connected to
    }

    fn make_test_join_request() -> SpaceData {
        SpaceData {
            /// Identifier of this request
            request_id: "foo_id".into(),
            space_address: "space_addr".into(),
            agent_id: "agent_id".into(),
        }
    }

    #[test]
    fn test_ghost_engine_join() {
        let mut lib3h = make_test_engine_wrapper("test_ghost_engine_join");

        let req_data = make_test_join_request();
        let result = lib3h.as_mut().handle_join(test_span(""), &req_data);
        assert!(result.is_ok());
        assert_eq!(lib3h.as_ref().space_gateway_map.len(), 1);
        let result = lib3h.as_mut().handle_join(test_span(""), &req_data);
        assert_eq!(
            "Err(Lib3hError(Other(\"Already joined space\")))",
            format!("{:?}", result)
        );
    }

    #[test]
    fn test_ghost_engine_leave() {
        let mut lib3h = make_test_engine_wrapper("test_ghost_engine_leave");
        let req_data = make_test_join_request();
        let result = lib3h.as_mut().handle_join(test_span(""), &req_data);
        assert!(result.is_ok());
        let result = lib3h.as_mut().handle_leave(test_span(""), &req_data);
        assert!(result.is_ok());
        let result = lib3h.as_mut().handle_leave(test_span(""), &req_data);
        assert_eq!(
            "Err(Lib3hError(Other(\"Not part of that space\")))",
            format!("{:?}", result)
        );
    }

    #[test]
    fn test_ghost_engine_dm() {
        let mut lib3h = make_test_engine_wrapper("test_ghost_engine_dm");
        let req_data = make_test_join_request();
        let result = lib3h.as_mut().handle_join(test_span(""), &req_data);
        assert!(result.is_ok());

        let direct_message = DirectMessageData {
            request_id: "foo_id".into(),
            space_address: "space_addr".into(),
            from_agent_id: "agent_id".into(),
            to_agent_id: "to_agent_id".into(),
            content: b"foo content".to_vec().into(),
        };

        let msg = GhostMessage::test_constructor();

        let result = lib3h
            .as_mut()
            .handle_direct_message(test_span(""), msg, &direct_message);
        assert!(result.is_ok());
        // TODO: assert somehow that the message got queued to the right place

        /*
            "Ok(DirectMessageData { space_address: HashString(\"space_addr\"), request_id: \"foo_id\", to_agent_id: HashString(\"agent_id\"), from_agent_id: HashString(\"to_agent_id\"), content: [102, 97, 107, 101, 32, 114, 101, 115, 112, 111, 110, 115, 101] })",
            format!("{:?}", result)
        );*/
    }

    fn make_test_entry() -> ProvidedEntryData {
        let aspect_list = Vec::new();
        let entry_data = EntryData {
            entry_address: "fake_address".into(),
            aspect_list,
        };
        ProvidedEntryData {
            space_address: "space_addr".into(),
            provider_agent_id: "agent_id".into(),
            entry: entry_data,
        }
    }

    #[test]
    fn test_ghost_engine_publish() {
        enable_logging_for_test(true);

        let mut engine = make_test_engine_wrapper("test_ghost_engine_publish");
        let req_data = make_test_join_request();
        let result = engine.as_mut().handle_join(test_span(""), &req_data);
        assert!(result.is_ok());

        let mut core = MockCore {
            //        state: "".to_string(),
        };

        let res = engine.process(&mut core);
        println!("engine.process() -> {:?}", res);

        let entry_data = make_test_entry();

        let result = engine
            .as_mut()
            .handle_publish_entry(test_span(""), &entry_data);
        assert!(result.is_ok());

        /* what should we observe to know that the entry was published?
        let space_gateway = lib3h
            .as_mut()
            .get_space(
                &req_data.space_address.to_owned(),
                &req_data.agent_id.to_owned(),
            )
            .unwrap();
        let msgs = space_gateway.as_mut().as_dht_mut().drain_messages();
        assert_eq!(msgs.len(), 0);

        {
            lib3h.process(&mut core).unwrap();
        }

        let space_gateway = lib3h
            .as_mut()
            .get_space(
                &req_data.space_address.to_owned(),
                &req_data.agent_id.to_owned(),
            )
            .unwrap();

        let msgs = space_gateway.as_mut().as_dht_mut().drain_messages();
        assert_eq!(
            "[GhostMessage {request_id: None, ..}]",
            format!("{:?}", msgs)
        ); */
    }

    #[test]
    fn test_ghost_engine_hold() {
        enable_logging_for_test(true);

        let mut lib3h = make_test_engine_wrapper("test_ghost_engine_hold");
        let req_data = make_test_join_request();
        let result = lib3h.as_mut().handle_join(test_span(""), &req_data);
        assert!(result.is_ok());

        let mut core = MockCore {
            //        state: "".to_string(),
        };

        let res = lib3h.process(&mut core);
        println!("engine.process() -> {:?}", res);

        let entry_data = make_test_entry();

        println!("Before handle_hold_entry ---------------------------");

        let result = lib3h.as_mut().handle_hold_entry(test_span(""), &entry_data);
        assert!(result.is_ok());

        /* what should we observe to know that the hold was published?
        let space_gateway = lib3h
            .as_mut()
            .get_space(
                &req_data.space_address.to_owned(),
                &req_data.agent_id.to_owned(),
            )
            .unwrap();
        let msgs = space_gateway.as_mut().as_dht_mut().drain_messages();
        assert_eq!(msgs.len(), 0);

        {
            lib3h.process(&mut core).unwrap();
        }

        let space_gateway = lib3h
            .as_mut()
            .get_space(
                &req_data.space_address.to_owned(),
                &req_data.agent_id.to_owned(),
            )
            .unwrap();

        let msgs = space_gateway.as_mut().as_dht_mut().drain_messages();
        for mut msg in msgs {
            let _payload = msg.take_message();
            assert_eq!(
                "dht publish",
                format!("{:?}", payload)
            );
        }*/
    }

    fn make_test_query(space_address: Address) -> QueryEntryData {
        QueryEntryData {
            space_address: space_address,
            entry_address: "fake_entry_address".into(),
            request_id: "fake_request_id".into(),
            requester_agent_id: "fake_requester_agent_id".into(),
            query: b"fake query".to_vec().into(),
        }
    }

    #[test]
    fn test_ghost_engine_query() {
        enable_logging_for_test(true);

        let mut lib3h = make_test_engine_wrapper("test_ghost_engine_query");
        let req_data = make_test_join_request();
        let result = lib3h.as_mut().handle_join(test_span(""), &req_data);
        assert!(result.is_ok());

        let mut core = MockCore {
            //        state: "".to_string(),
        };

        let res = lib3h.process(&mut core);
        println!("engine.process() -> {:?}", res);

        let query = make_test_query(req_data.space_address.clone());

        let _result = lib3h.request(
            test_span(""),
            ClientToLib3h::QueryEntry(query),
            Box::new(move |_me, _response| {
                panic!("BANG");
            }),
        );

        /*  FIXME: what should we observe to know that the query was processed
        let space_gateway = lib3h
            .as_mut()
            .get_space(
                &req_data.space_address.to_owned(),
                &req_data.agent_id.to_owned(),
            )
            .unwrap();
        let msgs = space_gateway.as_mut().as_dht_mut().drain_messages();
        assert_eq!(msgs.len(), 0);

        lib3h.process(&mut core).unwrap();

        let space_gateway = lib3h
            .as_mut()
            .get_space(
                &req_data.space_address.to_owned(),
                &req_data.agent_id.to_owned(),
            )
            .unwrap();

        let msgs = space_gateway.as_mut().as_dht_mut().drain_messages();
        for mut msg in msgs {
            let payload = msg.take_message();
            assert_eq!(
                "[GhostMessage {request_id: None, ..}]",
                format!("{:?}", payload)
            );
        }*/
    }
}
