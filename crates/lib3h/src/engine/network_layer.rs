use crate::{
    dht::dht_protocol::*,
    engine::{ghost_engine::handle_GossipTo, p2p_protocol::P2pProtocol, GhostEngine},
    error::{ErrorKind, Lib3hError, Lib3hResult},
    gateway::protocol::*,
    transport,
};

use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::*, protocol::*, uri::Lib3hUri, DidWork};
use lib3h_p2p_protocol::p2p::P2pMessage;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

/// Network layer related private methods
impl<'engine> GhostEngine<'engine> {
    /// Process whatever the network has in for us.
    pub(crate) fn process_multiplexer(&mut self) -> Lib3hResult<DidWork> {
        let mut did_work = false;
        // Process the network gateway
        detach_run!(&mut self.multiplexer, |ng| ng.process(self))?;
        for mut request in self.multiplexer.drain_messages() {
            did_work = true;
            // this should probably be changed so each arm has a different name span
            let span = request.span().child("process_multiplexer");
            let payload = request.take_message().expect("exists");
            match payload {
                GatewayRequestToParent::Dht(dht_request) => {
                    self.handle_network_dht_request(span, dht_request)?;
                }
                GatewayRequestToParent::Transport(transport_request) => {
                    self.handle_network_transport_request(span, &transport_request)?;
                }
            }
        }

        for (to, payload) in self.multiplexer_defered_sends.drain(..) {
            // println!("########## {} {}", to, payload);
            self.multiplexer.request(
                Span::fixme(),
                GatewayRequestToChild::Transport(
                    transport::protocol::RequestToChild::create_send_message(to, payload),
                ),
                Box::new(move |_me, _response| Ok(())),
            )?;
        }

        // Done
        Ok(did_work)
    }

    /// Handle a DhtRequestToParent sent to us by our network gateway
    fn handle_network_dht_request(
        &mut self,
        span: Span,
        request: DhtRequestToParent,
    ) -> Lib3hResult<()> {
        debug!("{} << handle_network_dht_request: {:?}", self.name, request);
        match request {
            DhtRequestToParent::GossipTo(gossip_data) => {
                handle_GossipTo(
                    self.config.network_id.id.clone(),
                    self.multiplexer.as_mut(),
                    gossip_data,
                )
                .expect("Failed to gossip with multiplexer");
            }
            DhtRequestToParent::GossipUnreliablyTo(_data) => {
                // no-op
            }
            DhtRequestToParent::HoldPeerRequested(peer_data) => {
                // TODO #167 - hardcoded for MirrorDHT and thus should not appear here.
                // Connect to every peer we are requested to hold.
                info!(
                    "{} auto-connect to peer: {} ({})",
                    self.name, peer_data.peer_name, peer_data.peer_location,
                );
                let cmd = GatewayRequestToChild::Transport(
                    transport::protocol::RequestToChild::create_send_message(
                        peer_data.peer_location,
                        Opaque::new(),
                    ),
                );
                self.multiplexer
                    .publish(span.child("DhtRequestToParent::HoldPeerRequested"), cmd)?;
            }
            DhtRequestToParent::PeerTimedOut(_peer_name) => {
                // Disconnect from that peer by calling a Close on it.
                // FIXME
            }
            // No entries in Network DHT
            DhtRequestToParent::HoldEntryRequested {
                from_peer_name: _,
                entry: _,
            } => {
                unreachable!();
            }
            DhtRequestToParent::EntryPruned(_) => {
                unreachable!();
            }
            DhtRequestToParent::RequestEntry(_) => {
                unreachable!();
            }
        }
        Ok(())
    }

    /// Handle a TransportRequestToParent sent to us by our network gateway
    #[allow(irrefutable_let_patterns)]
    fn handle_network_transport_request(
        &mut self,
        span: Span,
        request: &transport::protocol::RequestToParent,
    ) -> Lib3hResult<()> {
        debug!(
            "{} << handle_network_transport_request: {:?}",
            self.name, request
        );
        // Note: use same order as the enum
        match request {
            transport::protocol::RequestToParent::ErrorOccured { uri, error } => {
                if error.kind() == &transport::error::ErrorKind::Unbind {
                    let data = UnboundData { uri: uri.clone() };
                    self.lib3h_endpoint
                        .publish(Span::fixme(), Lib3hToClient::Unbound(data))?;
                } else {
                    // FIXME #391
                    error!("unhandled error {}", error);
                }
            }
            transport::protocol::RequestToParent::IncomingConnection { uri } => {
                self.handle_incoming_connection(
                    span.child("handle_incoming_connection"),
                    uri.clone(),
                )?;
            }
            transport::protocol::RequestToParent::ReceivedData { uri, payload } => {
                debug!("Received message from: {} | size: {}", uri, payload.len());
                // zero len() means its just a ping, no need to deserialize and handle
                if payload.len() == 0 {
                    debug!("Implement Ping!");
                } else {
                    let mut de = Deserializer::new(&payload[..]);
                    let maybe_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                        Deserialize::deserialize(&mut de);
                    if let Err(e) = maybe_msg {
                        error!("Failed deserializing msg: {:?}", e);
                        return Err(Lib3hError::new(ErrorKind::RmpSerdeDecodeError(e)));
                    }
                    let p2p_msg = maybe_msg.unwrap();
                    // debug!("p2p_msg: {:?}", p2p_msg);
                    self.serve_P2pProtocol(span.child("serve_P2pProtocol"), uri, p2p_msg)?;
                }
            }
        };
        Ok(())
    }

    fn defer_send(&mut self, to: Lib3hUri, payload: Opaque) {
        self.multiplexer_defered_sends.push((to, payload));
    }

    fn handle_incoming_connection(
        &mut self,
        span: Span,
        net_location: Lib3hUri,
    ) -> Lib3hResult<()> {
        // Get list of known peers
        let net_location_copy = net_location.clone();
        self.multiplexer
            .request(
                span.child("handle_incoming_connection, .request"),
                GatewayRequestToChild::Dht(DhtRequestToChild::RequestPeerList),
                Box::new(move |me, response| {
                    let response = {
                        match response {
                            GhostCallbackData::Timeout(bt) => panic!("timeout: {:?}", bt),
                            GhostCallbackData::Response(response) => match response {
                                Err(e) => panic!("{:?}", e),
                                Ok(response) => response,
                            },
                        }
                    };
                    if let GatewayRequestToChildResponse::Dht(
                        DhtRequestToChildResponse::RequestPeerList(peer_list),
                    ) = response
                    {
                        // TODO #150 - Should do this in next process instead
                        // Send to other node our Joined Spaces
                        {
                            let space_list = me.get_all_spaces();
                            let our_joined_space_list = P2pProtocol::AllJoinedSpaceList(space_list);
                            let mut payload = Vec::new();
                            our_joined_space_list
                                .serialize(&mut Serializer::new(&mut payload))
                                .unwrap();
                            trace!(
                                "AllJoinedSpaceList: {:?} to {:?}",
                                our_joined_space_list,
                                net_location_copy,
                            );
                            // we need a transportId, so search for it in the DHT
                            let maybe_peer_data = peer_list
                                .iter()
                                .find(|pd| pd.peer_location == net_location_copy);
                            trace!("--- got peerlist: {:?}", maybe_peer_data);
                            if let Some(peer_data) = maybe_peer_data {
                                trace!("AllJoinedSpaceList ; sending back to {:?}", peer_data);
                                me.defer_send(peer_data.peer_name.clone(), payload.into());
                                /* TODO: #777
                                me.multiplexer.publish(
                                    span.follower("publish TODO name"),
                                    GatewayRequestToChild::Transport(
                                        transport::protocol::RequestToChild::SendMessage {
                                            uri: &peer_data.peer_name,
                                            payload: payload.into(),
                                        },
                                    ),
                                )?;*/
                            }
                        }
                    // TODO END
                    } else {
                        panic!("bad response to RequestPeerList: {:?}", response);
                    }
                    Ok(())
                }),
            )
            .expect("sync functions should work");

        // Output a Lib3hToClient::Connected if its the first connection
        if self.network_connections.is_empty() {
            let data = ConnectedData {
                request_id: "".to_string(),
                uri: net_location.clone(),
            };
            self.lib3h_endpoint
                .publish(Span::fixme(), Lib3hToClient::Connected(data))?;
        }
        let _ = self.network_connections.insert(net_location.to_owned());
        Ok(())
    }

    /// Serve a P2pProtocol sent to us by the network.
    /// TODO #150
    #[allow(non_snake_case)]
    fn serve_P2pProtocol(
        &mut self,
        span: Span,
        _from: &Lib3hUri,
        p2p_msg: P2pProtocol,
    ) -> Lib3hResult<()> {
        match p2p_msg {
            P2pProtocol::Gossip(msg) => {
                // Prepare remoteGossipTo to post to dht
                let gossip = RemoteGossipBundleData {
                    from_peer_name: msg.from_peer_name.clone(),
                    bundle: msg.bundle.clone(),
                };
                // Check if its for the multiplexer
                if msg.space_address == self.config.network_id.id {
                    let _ = self.multiplexer.publish(
                        span.follower("TODO"),
                        GatewayRequestToChild::Dht(DhtRequestToChild::HandleGossip(gossip)),
                    );
                } else {
                    // otherwise should be for one of our space
                    let maybe_space_gateway = self.space_gateway_map.get_mut(&(
                        msg.space_address.to_owned(),
                        msg.to_peer_name.clone().into(),
                    ));
                    if let Some(space_gateway) = maybe_space_gateway {
                        let _ = space_gateway.publish(
                            span.follower("TODO"),
                            GatewayRequestToChild::Dht(DhtRequestToChild::HandleGossip(gossip)),
                        );
                    } else {
                        warn!("received gossip for unjoined space: {}", msg.space_address);
                    }
                }
            }
            P2pProtocol::DirectMessage(dm_data) => {
                // we got some data that should go up the multiplexer
                // let's try decoding it : )

                self.multiplexer
                    .as_mut()
                    .as_mut()
                    .received_data_for_agent_space_route(
                        &dm_data.space_address,
                        &dm_data.to_agent_id,
                        &dm_data.from_agent_id,
                        dm_data.content,
                    )?;
            }
            P2pProtocol::DirectMessageResult(_dm_data) => {
                panic!("we should never get a DirectMessageResult at this layer... only using DirectMessage");
            }
            P2pProtocol::PeerName(_, _, _) => {
                // no-op
            }
            P2pProtocol::BroadcastJoinSpace(gateway_id, peer_data) => {
                debug!("Received JoinSpace: {} {:?}", gateway_id, peer_data);
                for (_, space_gateway) in self.space_gateway_map.iter_mut() {
                    space_gateway.publish(
                        span.follower("P2pProtocol::BroadcastJoinSpace"),
                        GatewayRequestToChild::Dht(DhtRequestToChild::HoldPeer(peer_data.clone())),
                    )?;
                }
            }
            P2pProtocol::AllJoinedSpaceList(join_list) => {
                debug!("Received AllJoinedSpaceList: {:?}", join_list);
                for (space_address, peer_data) in join_list {
                    let maybe_space_gateway = self.get_first_space_mut(&space_address);
                    if let Some(space_gateway) = maybe_space_gateway {
                        let _ = space_gateway.publish(
                            span.follower("P2pProtocol::AllJoinedSpaceList"),
                            GatewayRequestToChild::Dht(DhtRequestToChild::HoldPeer(
                                peer_data.clone(),
                            )),
                        )?;
                    }
                }
            }
            P2pProtocol::CapnProtoMessage(bytes) => {
                match P2pMessage::from_bytes(bytes)? {
                    P2pMessage::MsgPing(ping) => {
                        unimplemented!();
                    }
                    P2pMessage::MsgPong(pong) => {
                        unimplemented!();
                    }
                }
            }
        };
        Ok(())
    }
}
