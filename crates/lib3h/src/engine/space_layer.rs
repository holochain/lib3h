#![allow(non_snake_case)]

use super::RealEngineTrackerData;
use crate::{
    dht::dht_protocol::*,
    engine::{ghost_engine::handle_GossipTo, p2p_protocol::*, ChainId, GhostEngine},
    error::*,
    gateway::{protocol::*, P2pGateway},
    transport::protocol::*,
};
use detach::prelude::*;
use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::*, protocol::*, uri::Lib3hUri, DidWork};
use rmp_serde::Deserializer;
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;

/// Space layer related private methods
/// Engine does not process a space gateway's Transport because it is shared with the network layer
impl<'engine> GhostEngine<'engine> {
    /// Return list of space+this_peer for all currently joined Spaces
    pub fn get_all_spaces(&mut self) -> Vec<(SpaceAddress, PeerData)> {
        let mut result = Vec::new();
        let chain_id_list: Vec<ChainId> = self
            .space_gateway_map
            .iter()
            .map(|(id, _)| id.clone())
            .collect();
        for chainId in chain_id_list {
            let space_address: String = chainId.0.clone().into();
            result.push((
                space_address,
                self.this_space_peer(chainId.clone())
                    .expect("Shouldn't find non-existing peer")
                    .clone(),
            ));
        }
        result
    }

    /// Return first space gateway for a specified space_address
    pub fn get_first_space_mut(
        &mut self,
        space_address: &str,
    ) -> Option<&mut GatewayParentWrapper<GhostEngine<'engine>, P2pGateway>> {
        for (chainId, space_gateway) in self.space_gateway_map.iter_mut() {
            let current_space_address: String = chainId.0.clone().into();
            if current_space_address == space_address {
                return Some(space_gateway);
            }
        }
        None
    }

    /// Process all space gateways
    pub(crate) fn process_space_gateways(&mut self) -> Lib3hResult<DidWork> {
        // Process all space gateways and collect requests
        let mut space_outbox_map = HashMap::new();
        let mut space_gateway_map: HashMap<
            ChainId,
            Detach<GatewayParentWrapper<GhostEngine<'engine>, P2pGateway>>,
        > = self.space_gateway_map.drain().collect();
        let mut did_work = false;
        for (chain_id, mut space_gateway) in space_gateway_map.drain() {
            detach_run!(space_gateway, |g| g.process(self))?;
            let request_list = space_gateway.drain_messages();
            did_work = did_work || request_list.len() > 0;
            space_outbox_map.insert(chain_id.clone(), request_list);
            self.space_gateway_map.insert(chain_id, space_gateway);
        }
        // Process all space gateway requests
        for (chain_id, request_list) in space_outbox_map {
            for request in request_list {
                self.handle_space_request(
                    request.span().child("handle_space_request"),
                    &chain_id,
                    request,
                )?;
            }
        }
        // Done
        Ok(did_work)
    }

    /// Handle a GatewayRequestToParent sent to us by one of our space gateway
    fn handle_space_request(
        &mut self,
        span: Span,
        chain_id: &ChainId,
        mut request: GatewayToParentMessage,
    ) -> Lib3hResult<DidWork> {
        debug!(
            "{} << handle_space_request: [{:?}] - {:?}",
            self.name, chain_id, request,
        );
        let space_gateway = self
            .space_gateway_map
            .get_mut(chain_id)
            .expect("Should have the space gateway we receive an event from.");
        let payload = request.take_message().expect("exists");
        match payload {
            // Handle Space's DHT request
            // ==========================
            GatewayRequestToParent::Dht(dht_request) => {
                match dht_request {
                    DhtRequestToParent::GossipTo(gossip_data) => {
                        let from_peer_name = &space_gateway.as_mut().as_mut().this_peer().peer_name;
                        handle_GossipTo(
                            chain_id.0.clone(),
                            space_gateway,
                            from_peer_name,
                            gossip_data,
                        )
                        .expect("Failed to gossip with space_gateway");
                    }
                    DhtRequestToParent::GossipUnreliablyTo(_data) => {
                        // n/a - should have been handled by gateway
                    }
                    // HoldPeerRequested from gossip
                    DhtRequestToParent::HoldPeerRequested(peer_data) => {
                        debug!(
                            "{} -- ({}).post() HoldPeer {:?}",
                            self.name, chain_id.0, peer_data,
                        );
                        // For now accept all request
                        let _res = space_gateway.publish(
                            span.follower("DhtRequestToParent::HoldPeerRequested"),
                            GatewayRequestToChild::Dht(DhtRequestToChild::HoldPeer(peer_data)),
                        );
                    }
                    DhtRequestToParent::PeerTimedOut(_peer_name) => {
                        // no-op
                    }
                    // HoldEntryRequested from gossip
                    // -> Send each aspect to Core for validation
                    DhtRequestToParent::HoldEntryRequested {
                        from_peer_name,
                        entry,
                    } => {
                        for aspect in entry.aspect_list {
                            let lib3h_msg = StoreEntryAspectData {
                                request_id: self.request_track.reserve(),
                                space_address: chain_id.0.clone(),
                                provider_agent_id: from_peer_name.clone().into(),
                                entry_address: entry.entry_address.clone(),
                                entry_aspect: aspect,
                            };
                            // TODO - not sure what core is expected to send back here
                            //      - right now these tracks will timeout
                            self.request_track.set(
                                &lib3h_msg.request_id,
                                Some(RealEngineTrackerData::HoldEntryRequested),
                            );
                            self.lib3h_endpoint.publish(
                                Span::fixme(),
                                Lib3hToClient::HandleStoreEntryAspect(lib3h_msg),
                            )?;
                        }
                    }
                    DhtRequestToParent::EntryPruned(_address) => {
                        // TODO #174
                    }
                    // EntryDataRequested: Change it into a Lib3hToClient::HandleFetchEntry.
                    DhtRequestToParent::RequestEntry(entry_address) => {
                        let msg = FetchEntryData {
                            space_address: chain_id.0.clone(),
                            entry_address: entry_address.clone(),
                            request_id: "FIXME".to_string(),
                            provider_agent_id: chain_id.1.clone(),
                            aspect_address_list: None,
                        };
                        self.lib3h_endpoint
                            .request(
                                Span::fixme(),
                                Lib3hToClient::HandleFetchEntry(msg.clone()),
                                Box::new(move |me, response| {
                                    let mut is_data_for_author_list = false;
                                    if me.request_track.has(&msg.request_id) {
                                        match me.request_track.remove(&msg.request_id) {
                                            Some(data) => match data {
                                                RealEngineTrackerData::DataForAuthorEntry => {
                                                    is_data_for_author_list = true;
                                                }
                                                _ => (),
                                            },
                                            None => (),
                                        };
                                    }
                                    let maybe_space = me.get_space(
                                        &msg.space_address,
                                        &msg.provider_agent_id,
                                    );
                                    match maybe_space {
                                        Err(_res) => {
                                            debug!("Received response to our HandleFetchEntry for a space we are not part of anymore");
                                        },
                                        Ok(space_gateway) => {
                                            let entry = match response {
                                                GhostCallbackData::Response(Ok(Lib3hToClientResponse::HandleFetchEntryResult(msg))) => {
                                                    msg.entry
                                                }
                                                GhostCallbackData::Response(Err(e)) => {
                                                    panic!("Got error on HandleFetchEntry: {:?} ", e);
                                                }
                                                GhostCallbackData::Timeout(bt) => {
                                                    panic!("Got timeout on HandleFetchEntry: {:?}", bt);
                                                }
                                                _ => panic!("bad response type"),
                                            };
                                            if is_data_for_author_list {
                                                space_gateway.publish(
                                                    Span::fixme(),
                                                    GatewayRequestToChild::Dht(DhtRequestToChild::BroadcastEntry(entry)))?;
                                            } else {
                                                request.respond(Ok(
                                                    GatewayRequestToParentResponse::Dht(DhtRequestToParentResponse::RequestEntry(entry),
                                                    )))?;
                                            }
                                        }
                                    }
                                    Ok(())
                                }))
                            ?;
                    }
                }
            }
            // Handle Space's Transport request
            // ================================
            GatewayRequestToParent::Transport(transport_request) => {
                trace!("space_layer got {:#?}", transport_request);
                match transport_request {
                    RequestToParent::ErrorOccured { uri: _, error } => {
                        panic!("can't handle {:?}", error);
                    }
                    RequestToParent::IncomingConnection { uri } => {
                        panic!("can't handle incoming connection {:?}", uri);
                    }
                    RequestToParent::ReceivedData { uri, payload } => {
                        if payload.len() == 0 {
                            debug!("Implement Ping!");
                        } else {
                            let mut de = Deserializer::new(&payload[..]);
                            let maybe_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                                Deserialize::deserialize(&mut de);
                            if let Err(e) = maybe_msg {
                                error!("Failed deserializing msg: {:?}", e);
                                return Err(e.into());
                            }
                            let p2p_msg = maybe_msg.unwrap();
                            trace!("space_layer about to handle p2p_msg: {:?}", p2p_msg);
                            self.handle_p2p_protocol(
                                span.child("handle_p2p_protocol"),
                                &uri,
                                p2p_msg,
                            )?;
                        }
                    }
                }
            }
        }
        Ok(true /* fixme */)
    }

    /// process P2pProtocol messages that have bubbled up to the space_layer
    fn handle_p2p_protocol(
        &mut self,
        span: Span,
        _from: &Url,
        p2p_msg: P2pProtocol,
    ) -> Lib3hResult<()> {
        match p2p_msg {
            P2pProtocol::DirectMessage(dm_data) => {
                self.lib3h_endpoint.request(
                    span,
                    Lib3hToClient::HandleSendDirectMessage(dm_data),
                    Box::new(move |me, resp| match resp {
                        GhostCallbackData::Response(Ok(
                            Lib3hToClientResponse::HandleSendDirectMessageResult(dm_data),
                        )) => {
                            let to_agent_id = dm_data.to_agent_id.clone();
                            let (space_gateway, payload) = match me.prepare_direct_peer_msg(
                                dm_data.space_address.clone(),
                                dm_data.from_agent_id.clone(),
                                dm_data.to_agent_id.clone(),
                                P2pProtocol::DirectMessageResult(dm_data.clone()),
                            ) {
                                Ok(r) => r,
                                Err(e) => panic!("{:?}", e),
                            };
                            trace!(
                                "handle_p2p_protocol: Got p2p_msg for {}: {:?}",
                                to_agent_id.clone(),
                                dm_data
                            );

                            space_gateway.publish(
                                Span::fixme(),
                                GatewayRequestToChild::Transport(
                                    RequestToChild::create_send_message(
                                        Lib3hUri::with_agent_id(&to_agent_id),
                                        payload,
                                    ),
                                ),
                            )?;
                            Ok(())
                        }
                        _ => panic!("unhandled: {:?}", resp),
                    }),
                )?;
            }
            P2pProtocol::DirectMessageResult(dm_data) => {
                trace!(
                    "pending_client_messages: {:?}",
                    self.pending_client_direct_messages
                );
                if let Some(msg) = self
                    .pending_client_direct_messages
                    .remove(&dm_data.request_id)
                {
                    trace!(
                        "space_layer respond with send direct message result: {:?}",
                        dm_data
                    );
                    msg.respond(Ok(ClientToLib3hResponse::SendDirectMessageResult(dm_data)))?;
                } else {
                    error!(
                        "space_layer couldn't find message with request_id sourced from {:?}",
                        dm_data
                    );
                }
            }
            P2pProtocol::Gossip(gossip_data) => {
                let remote_gossip = RemoteGossipBundleData {
                    from_peer_name: gossip_data.from_peer_name.clone(),
                    bundle: gossip_data.bundle.clone(),
                };
                let space_gateway =
                    self.get_space(&gossip_data.space_address, &gossip_data.to_peer_name.into())?;
                space_gateway.publish(
                    span.follower("TODO"),
                    GatewayRequestToChild::Dht(DhtRequestToChild::HandleGossip(remote_gossip)),
                )?;
            }
            _ => {
                panic!("can't handle space layer receive of {:?}", p2p_msg);
            }
        };
        Ok(())
    }
}
