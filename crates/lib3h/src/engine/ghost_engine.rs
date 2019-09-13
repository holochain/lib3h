use detach::Detach;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::*, protocol::*, Address};
use std::collections::{HashMap, HashSet};

use crate::{
    dht::{dht_config::DhtConfig, dht_protocol::*},
    engine::{
        p2p_protocol::{GossipData, P2pProtocol},
        ChainId, EngineConfig, GhostEngine, TransportKeys, NETWORK_GATEWAY_ID,
    },
    error::{ErrorKind, Lib3hError, Lib3hResult},
    gateway::{protocol::*, P2pGateway},
    track::Tracker,
    transport::{self, memory_mock::ghost_transport_memory::*, TransportMultiplex},
};
use lib3h_crypto_api::{Buffer, CryptoSystem};
use lib3h_tracing::Lib3hSpan;
use rmp_serde::Serializer;
use serde::Serialize;
use url::Url;

pub type ClientToLib3hMessage =
    GhostMessage<ClientToLib3h, Lib3hToClient, ClientToLib3hResponse, Lib3hError>;

pub type Lib3hToClientMessage =
    GhostMessage<Lib3hToClient, ClientToLib3h, Lib3hToClientResponse, Lib3hError>;

/// this is a generic parent wrapper for a GhostEngine.  This allows us to have
/// a mock GhostEngine for proving out the LegacyLib3h wrapper
pub type GhostEngineParentWrapper<Core, Engine, EngineError> = GhostParentWrapper<
    Core,
    Lib3hToClient,
    Lib3hToClientResponse,
    ClientToLib3h,
    ClientToLib3hResponse,
    EngineError,
    Engine,
>;

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

impl<'engine> GhostEngine<'engine> {
    pub fn advertise(&self) -> Url {
        self.this_net_peer.peer_uri.to_owned()
    }

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
            GhostTransportMemory::new(),
        )
    }

    pub fn with_transport(
        crypto: Box<dyn CryptoSystem>,
        config: EngineConfig,
        name: &str,
        dht_factory: DhtFactory,
        mut transport: GhostTransportMemory, // FIXME: TEMPORARY hardcoded to memory
    ) -> Lib3hResult<Self> {
        let memory_network_endpoint = Detach::new(
            transport
                .take_parent_endpoint()
                .expect("exists")
                .as_context_endpoint_builder()
                .request_id_prefix("tmem_to_child_")
                .build::<P2pGateway>(),
        );

        /*
        // Bind & create this_net_peer
        // TODO: Find better way to do init with GhostEngine
        memory_network_endpoint.request(
            Lib3hTrace,
            transport::protocol::RequestToChild::Bind {
                spec: config.bind_url.clone(),
            },
            Box::new(|mut ud, response| {
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => match response {
                            Err(e) => panic!("{:?}", e),
                            Ok(response) => response,
                        },
                    }
                };
                if let transport::protocol::RequestToChildResponse::Bind(bind_data) = response {
                    ud.binding = bind_data.bound_url;
                } else {
                    panic!("bad response to bind: {:?}", response);
                }
                Ok(())
            }),
        )?;
        transport.process()?;
        memory_network_endpoint.process(&mut gateway_ud)?;
        */
        let fixme_binding = Url::parse("fixme::host:123").unwrap();
        let this_net_peer = PeerData {
            peer_address: format!("{}_tId", name),
            peer_uri: fixme_binding.clone(),
            timestamp: 0, // TODO #166
        };
        // Create DhtConfig
        let dht_config =
            DhtConfig::with_engine_config(&format!("{}_tId", name), &fixme_binding, &config);
        debug!("New MOCK Engine {} -> {:?}", name, this_net_peer);
        let transport_keys = TransportKeys::new(crypto.as_crypto_system())?;
        let multiplexer = Detach::new(GatewayParentWrapper::new(
            TransportMultiplex::new(P2pGateway::new(
                NETWORK_GATEWAY_ID,
                memory_network_endpoint,
                dht_factory,
                &dht_config,
            )),
            "to_multiplexer_",
        ));
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
            process_count: 0,
            client_endpoint: Some(endpoint_parent),
            lib3h_endpoint: Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix(name)
                    .build(),
            ),
        };

        engine.priv_connect_bootstraps()?;
        Ok(engine)
    }

    fn priv_connect_bootstraps(&mut self) -> GhostResult<()> {
        let nodes: Vec<Url> = self.config.bootstrap_nodes.drain(..).collect();
        for bs in nodes {
            self.handle_connect(ConnectData {
                request_id: format!("bootstrap-connect: {}", bs.clone()).to_string(), // fire-and-forget
                peer_uri: bs,
                network_id: "".to_string(), // unimplemented
            })?;
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

        // process any messages from the client to us
        for msg in self.lib3h_endpoint.as_mut().drain_messages() {
            self.handle_msg_from_client(msg)?;
        }

        /* TODO: #324
                let outbox: Vec<Lib3hServerProtocol> = Vec::new();
                let (net_did_work, mut net_outbox) = self.process_multiplexer()?;
                outbox.append(&mut net_outbox);
                // Process the space layer
                let mut p2p_output = self.process_space_gateways()?;
                outbox.append(&mut p2p_output);
                // Hack
                let (ugly_did_work, mut ugly_outbox) = self.process_ugly();
                outbox.append(&mut ugly_outbox);
        */

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
    fn handle_connect(&mut self, data: ConnectData) -> GhostResult<()> {
        let cmd =
            GatewayRequestToChild::Transport(transport::protocol::RequestToChild::SendMessage {
                uri: data.peer_uri,
                payload: Opaque::new(),
            });
        // TODO: #339 convert to request and respond with ConnectedData
        self.multiplexer.publish(Lib3hSpan::todo(), cmd)
    }

    /// Process any Client events or requests
    fn handle_msg_from_client(&mut self, mut msg: ClientToLib3hMessage) -> GhostResult<()> {
        match msg.take_message().expect("exists") {
            ClientToLib3h::Connect(data) => {
                trace!("ClientToLib3h::Connect: {:?}", &data);
                self.handle_connect(data)
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

        // First create DhtConfig for space gateway
        let this_peer_transport_id_as_uri = {
            // TODO #175 - encapsulate this conversion logic
            Url::parse(format!("transportId:{}", self.this_net_peer.peer_address).as_str())
                .expect("can parse url")
        };
        let dht_config = DhtConfig::with_engine_config(
            &agent_id.to_string(),
            &this_peer_transport_id_as_uri,
            &self.config,
        );

        // Create new space gateway for this ChainId
        let uniplex_endpoint = Detach::new(
            self.multiplexer
                .as_mut()
                .as_mut()
                .create_agent_space_route(&space_address, &agent_id)
                .as_context_endpoint_builder()
                .build::<P2pGateway>(),
        );
        let new_space_gateway = GatewayParentWrapper::new(
            P2pGateway::new_with_space(
                &space_address,
                uniplex_endpoint,
                self.dht_factory,
                &dht_config,
            ),
            "space_gateway_",
        );
        self.space_gateway_map
            .insert(chain_id.clone(), Detach::new(new_space_gateway));
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
        let _space_gateway = self.get_space(
            &msg.space_address.to_owned(),
            &msg.provider_agent_id.to_owned(),
        )?;
        /* TODO: #326
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
            // Create "fake" entry, in the sense an entry with no actual content,
            // but valid addresses.
            let fake_entry = EntryData {
                entry_address: entry_address.clone(),
                aspect_list,
            };
            space_gateway
                .publish(DhtRequestToChild::HoldEntryAspectAddress(fake_entry))
                .map_err(|e| Lib3hError::new_other(&e.to_string()))?;
        }*/
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
    fn get_space(
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
    use url::Url;

    struct MockCore {
        //    state: String,
    }

    fn make_test_engine() -> GhostEngine<'static> {
        let crypto = Box::new(SodiumCryptoSystem::new());
        let config = EngineConfig {
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

        let engine: GhostEngine =
            GhostEngine::new_mock(crypto, config, "test_engine", dht_factory).unwrap();
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

        let mut lib3h = make_test_engine_wrapper();
        let req_data = make_test_join_request();
        let result = lib3h.as_mut().handle_join(&req_data);
        assert!(result.is_ok());

        let mut core = MockCore {
            //        state: "".to_string(),
        };

        lib3h.process(&mut core).unwrap();

        let entry_data = make_test_entry();

        let result = lib3h.as_mut().handle_publish_entry(&entry_data);
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

        lib3h.process(&mut core).unwrap();

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

        lib3h.process(&mut core).unwrap();

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
