use detach::Detach;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::*, protocol::*, Address};
use std::collections::{HashMap, HashSet};

use crate::{
    dht::{dht_config::DhtConfig, dht_protocol::*},
    engine::{
        engine_actor::*, p2p_protocol::*, CanAdvertise, ChainId, EngineConfig, GhostEngine,
        TransportKeys, NETWORK_GATEWAY_ID,
    },
    error::{ErrorKind, Lib3hError, Lib3hResult},
    gateway::{protocol::*, P2pGateway},
    keystore::KeystoreStub,
    track::Tracker,
    transport::{
        self, memory_mock::ghost_transport_memory::*, protocol::*, TransportEncoding,
        TransportMultiplex,
    },
};
use lib3h_crypto_api::CryptoSystem;
use lib3h_tracing::Lib3hSpan;
use rmp_serde::Serializer;
use serde::Serialize;
use url::Url;

#[allow(non_snake_case)]
pub fn handle_gossipTo<
    'engine,
    G: GhostActor<
        GatewayRequestToParent,
        GatewayRequestToParentResponse,
        GatewayRequestToChild,
        GatewayRequestToChildResponse,
        Lib3hError,
    >,
>(
    gateway_identifier: &str,
    gateway: &mut GatewayParentWrapper<GhostEngine<'engine>, G>,
    gossip_data: GossipToData,
) -> Lib3hResult<()> {
    debug!(
        "({}) handle_gossipTo: {:?}",
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
            space_address: gateway_identifier.into(),
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
        gateway.publish(Lib3hSpan::todo(), GatewayRequestToChild::Transport(msg))?;
    }
    Ok(())
}

impl<'engine> CanAdvertise for GhostEngine<'engine> {
    fn advertise(&self) -> Url {
        self.this_net_peer.peer_uri.to_owned()
    }
}
impl<'engine> GhostEngine<'engine> {
    /// Constructor with TransportMemory
    pub fn new_mock(
        crypto: Box<dyn CryptoSystem>,
        config: EngineConfig,
        name: &str,
        dht_factory: DhtFactory,
    ) -> Lib3hResult<Self> {
        // Create TransportMemory as the network transport
        Self::with_transport(
            crypto,
            config,
            name,
            dht_factory,
            Box::new(GhostTransportMemory::new()),
        )
    }

    pub fn with_transport(
        crypto: Box<dyn CryptoSystem>,
        config: EngineConfig,
        name: &str,
        dht_factory: DhtFactory,
        transport: DynTransportActor,
    ) -> Lib3hResult<Self> {
        let transport_keys = TransportKeys::new(crypto.as_crypto_system())?;
        let transport = TransportEncoding::new(
            crypto.box_clone(),
            transport_keys.transport_id.clone(),
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
                NETWORK_GATEWAY_ID,
                Box::new(transport),
                dht_factory,
                &dht_config,
            )),
            "to_multiplexer_",
        ));

        // Bind & create this_net_peer
        // TODO: Find better way to do init with GhostEngine
        multiplexer.as_mut().request(
            Lib3hSpan::todo(),
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
            Lib3hSpan::todo(),
            GatewayRequestToChild::Dht(DhtRequestToChild::UpdateAdvertise(
                engine.this_net_peer.peer_uri.clone(),
            )),
        )?;
        detach_run!(engine.multiplexer, |e| e.process(&mut engine))?;
        engine.priv_connect_bootstraps()?;
        Ok(engine)
    }

    fn priv_connect_bootstraps(&mut self) -> GhostResult<()> {
        let nodes: Vec<Url> = self.config.bootstrap_nodes.drain(..).collect();
        for bs in nodes {
            // can't use handle_bootstrap() because it assumes a message to respond to
            let cmd = GatewayRequestToChild::Transport(
                transport::protocol::RequestToChild::SendMessage {
                    uri: bs,
                    payload: Opaque::new(),
                },
            );
            self.multiplexer.publish(Lib3hSpan::todo(), cmd)?;
        }
        Ok(())
    }

    pub fn this_space_peer(&mut self, chain_id: ChainId) -> PeerData {
        trace!("engine.this_space_peer() ...");
        let space_gateway = self
            .space_gateway_map
            .get_mut(&chain_id)
            .expect("No space at chainId");
        space_gateway.as_mut().as_mut().this_peer()
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
            Lib3hSpan::todo(),
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
        match msg.take_message().expect("exists") {
            ClientToLib3h::Bootstrap(data) => {
                trace!("ClientToLib3h::Bootstrap: {:?}", &data);
                self.handle_bootstrap(msg, data)
            }
            ClientToLib3h::JoinSpace(data) => {
                trace!("ClientToLib3h::JoinSpace: {:?}", data);
                let result = self
                    .handle_join(&data)
                    .map(|_| ClientToLib3hResponse::JoinSpaceResult);
                msg.respond(result)
            }
            ClientToLib3h::LeaveSpace(data) => {
                trace!("ClientToLib3h::LeaveSpace: {:?}", data);
                let result = self
                    .handle_leave(&data)
                    .map(|_| ClientToLib3hResponse::LeaveSpaceResult);
                msg.respond(result)
            }
            ClientToLib3h::SendDirectMessage(data) => {
                trace!("ClientToLib3h::SendDirectMessage: {:?}", data);
                self.handle_direct_message(&data, false)
                    .map_err(|e| GhostError::from(e.to_string()))
            }
            ClientToLib3h::PublishEntry(data) => {
                trace!("ClientToLib3h::PublishEntry: {:?}", data);
                self.handle_publish_entry(&data)
                    .map_err(|e| GhostError::from(e.to_string()))
            }
            ClientToLib3h::HoldEntry(data) => {
                trace!("ClientToLib3h::HoldEntry: {:?}", data);
                self.handle_hold_entry(&data)
                    .map_err(|e| GhostError::from(e.to_string()))
            }
            ClientToLib3h::QueryEntry(data) => {
                trace!("ClientToLib3h::QueryEntry: {:?}", data);
                self.handle_query_entry(msg, data)
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
            Box::new(KeystoreStub::new()),
            Box::new(uniplex),
        );
        let new_space_gateway = Detach::new(GatewayParentWrapper::new(
            P2pGateway::new_with_space(
                &space_address,
                Box::new(uniplex),
                self.dht_factory,
                &dht_config,
            ),
            "space_gateway_",
        ));
        self.space_gateway_map
            .insert(chain_id.clone(), new_space_gateway);
        Ok(chain_id)
    }

    fn broadcast_join(&mut self, space_address: Address, peer: PeerData) -> GhostResult<()> {
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
            .publish(Lib3hSpan::todo(), GatewayRequestToChild::SendAll(payload))
        // TODO END
    }

    #[allow(non_snake_case)]
    fn handle_HandleGetAuthoringEntryListResult(&mut self, _msg: EntryListData) -> Lib3hResult<()> {
        /* TODO: #327

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
                              let _msg_data = FetchEntryData {
                                  space_address: msg.space_address.clone(),
                                  entry_address: entry_address.clone(),
                                  request_id,
                                  provider_agent_id: msg.provider_agent_id.clone(),
                                  aspect_address_list: None,
                              };

                              let _context = RequestContext {
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
                          }
                      } else {
                          panic!("bad response to RequestAspectsOf: {:?}", response);
                      }
                      Ok(())
                  }),
              )?;
          }*/
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
                    Lib3hSpan::todo(),
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
    fn handle_join(&mut self, join_msg: &SpaceData) -> Lib3hResult<()> {
        let chain_id =
            self.add_gateway(join_msg.space_address.clone(), join_msg.agent_id.clone())?;

        let this_peer = self.this_space_peer(chain_id.clone());
        self.broadcast_join(join_msg.space_address.clone(), this_peer.clone())?;

        let space_gateway = self.space_gateway_map.get_mut(&chain_id).unwrap();

        // Have DHT broadcast our PeerData
        space_gateway.publish(
            Lib3hSpan::todo(),
            GatewayRequestToChild::Dht(DhtRequestToChild::HoldPeer(this_peer)),
        )?;

        // Send Get*Lists requests
        let mut list_data = GetListData {
            space_address: join_msg.space_address.clone(),
            provider_agent_id: join_msg.agent_id.clone(),
            request_id: self.request_track.reserve(),
        };

        self.lib3h_endpoint.request(
            Lib3hSpan::todo(),
            Lib3hToClient::HandleGetGossipingEntryList(list_data.clone()),
            Box::new(|me, response| {
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
                Lib3hSpan::todo(),
                Lib3hToClient::HandleGetAuthoringEntryList(list_data.clone()),
                Box::new(|me, response| {
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
    ) -> Lib3hResult<()> {
        let chain_id = (msg.space_address.clone(), msg.from_agent_id.clone());

        let this_peer = self.this_space_peer(chain_id.clone());

        let to_agent_id: String = msg.to_agent_id.clone().into();
        if &this_peer.peer_address == &to_agent_id {
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
        let peer_address: String = msg.to_agent_id.clone().into();

        let space_gateway = self
            .space_gateway_map
            .get_mut(&chain_id)
            .ok_or_else(|| Lib3hError::new_other("Not part of that space"))?;

        space_gateway
            .publish(
                Lib3hSpan::todo(),
                GatewayRequestToChild::Transport(
                    transport::protocol::RequestToChild::SendMessage {
                        uri: Url::parse(&("agentId:".to_string() + &peer_address))
                            .expect("invalid url format"),
                        payload: payload.into(),
                    },
                ),
            )
            .map_err(|e| Lib3hError::new_other(&e.to_string()))
    }

    fn handle_publish_entry(&mut self, msg: &ProvidedEntryData) -> Lib3hResult<()> {
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
                Lib3hSpan::todo(),
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
                Lib3hSpan::todo(),
                GatewayRequestToChild::Dht(DhtRequestToChild::BroadcastEntry(msg.entry.clone())),
            )
            .map_err(|e| Lib3hError::new_other(&e.to_string()))
    }

    fn handle_hold_entry(&mut self, msg: &ProvidedEntryData) -> Lib3hResult<()> {
        let space_gateway = self.get_space(
            &msg.space_address.to_owned(),
            &msg.provider_agent_id.to_owned(),
        )?;
        space_gateway
            .publish(
                Lib3hSpan::todo(),
                GatewayRequestToChild::Dht(DhtRequestToChild::HoldEntryAspectAddress(
                    msg.entry.clone(),
                )),
            )
            .map_err(|e| Lib3hError::new_other(&e.to_string()))
    }

    fn handle_query_entry(
        &mut self,
        msg: ClientToLib3hMessage,
        data: QueryEntryData,
    ) -> Lib3hResult<()> {
        // TODO #169 reflecting for now...
        // ultimately this should get forwarded to the
        // correct neighborhood
        self.lib3h_endpoint
            .request(
                Lib3hSpan::todo(),
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
    gateway_identifier: &str,
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
            space_address: gateway_identifier.into(),
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
        gateway.publish(Lib3hSpan::todo(), GatewayRequestToChild::Transport(msg))?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dht::mirror_dht::MirrorDht, tests::enable_logging_for_test};
    use lib3h_sodium::SodiumCryptoSystem;
    use std::path::PathBuf;
    use url::Url;

    struct MockCore {
        //    state: String,
    }

    fn make_test_engine() -> GhostEngine<'static> {
        let crypto = Box::new(SodiumCryptoSystem::new());
        let config = EngineConfig {
            socket_type: "mem".into(),
            bootstrap_nodes: vec![],
            work_dir: PathBuf::new(),
            log_level: 'd',
            bind_url: Url::parse(format!("mem://{}", "test_engine").as_str()).unwrap(),
            dht_gossip_interval: 100,
            dht_timeout_threshold: 1000,
            dht_custom_config: vec![],
        };
        let dht_factory = MirrorDht::new_with_config;

        let engine = GhostEngine::new_mock(crypto, config, "test_engine", dht_factory).unwrap();
        engine
    }

    fn make_test_engine_wrapper(
    ) -> GhostEngineParentWrapper<MockCore, GhostEngine<'static>, Lib3hError> {
        let engine = make_test_engine();
        let lib3h: GhostEngineParentWrapper<MockCore, GhostEngine, Lib3hError> =
            GhostParentWrapper::new(engine, "test_engine");
        lib3h
    }

    #[test]
    fn test_ghost_engine_construct() {
        let lib3h = make_test_engine_wrapper();
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
        let mut lib3h = make_test_engine_wrapper();

        let req_data = make_test_join_request();
        let result = lib3h.as_mut().handle_join(&req_data);
        assert!(result.is_ok());
        assert_eq!(lib3h.as_ref().space_gateway_map.len(), 1);
        let result = lib3h.as_mut().handle_join(&req_data);
        assert_eq!(
            "Err(Lib3hError(Other(\"Already joined space\")))",
            format!("{:?}", result)
        );
    }

    #[test]
    fn test_ghost_engine_leave() {
        let mut lib3h = make_test_engine_wrapper();
        let req_data = make_test_join_request();
        let result = lib3h.as_mut().handle_join(&req_data);
        assert!(result.is_ok());
        let result = lib3h.as_mut().handle_leave(&req_data);
        assert!(result.is_ok());
        let result = lib3h.as_mut().handle_leave(&req_data);
        assert_eq!(
            "Err(Lib3hError(Other(\"Not part of that space\")))",
            format!("{:?}", result)
        );
    }

    #[test]
    fn test_ghost_engine_dm() {
        let mut lib3h = make_test_engine_wrapper();
        let req_data = make_test_join_request();
        let result = lib3h.as_mut().handle_join(&req_data);
        assert!(result.is_ok());

        let direct_message = DirectMessageData {
            request_id: "foo_id".into(),
            space_address: "space_addr".into(),
            from_agent_id: "agent_id".into(),
            to_agent_id: "to_agent_id".into(),
            content: b"foo content".to_vec().into(),
        };

        let result = lib3h.as_mut().handle_direct_message(&direct_message, false);
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

        let mut engine = make_test_engine_wrapper();
        let req_data = make_test_join_request();
        let result = engine.as_mut().handle_join(&req_data);
        assert!(result.is_ok());

        let mut core = MockCore {
            //        state: "".to_string(),
        };

        let res = engine.process(&mut core);
        println!("engine.process() -> {:?}", res);

        let entry_data = make_test_entry();

        let result = engine.as_mut().handle_publish_entry(&entry_data);
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

        let mut lib3h = make_test_engine_wrapper();
        let req_data = make_test_join_request();
        let result = lib3h.as_mut().handle_join(&req_data);
        assert!(result.is_ok());

        let mut core = MockCore {
            //        state: "".to_string(),
        };

        let res = lib3h.process(&mut core);
        println!("engine.process() -> {:?}", res);

        let entry_data = make_test_entry();

        println!("Before handle_hold_entry ---------------------------");

        let result = lib3h.as_mut().handle_hold_entry(&entry_data);
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

        let mut lib3h = make_test_engine_wrapper();
        let req_data = make_test_join_request();
        let result = lib3h.as_mut().handle_join(&req_data);
        assert!(result.is_ok());

        let mut core = MockCore {
            //        state: "".to_string(),
        };

        let res = lib3h.process(&mut core);
        println!("engine.process() -> {:?}", res);

        let query = make_test_query(req_data.space_address.clone());

        let _result = lib3h.request(
            Lib3hSpan::todo(),
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
