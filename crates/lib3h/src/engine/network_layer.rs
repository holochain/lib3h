use crate::{
    dht::dht_protocol::*,
    engine::{
        ghost_engine::handle_gossip_to, p2p_protocol::P2pProtocol, GhostEngine, NETWORK_GATEWAY_ID,
    },
    error::{ErrorKind, Lib3hError, Lib3hResult},
    gateway::protocol::*,
    transport,
};

use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::*, protocol::*, DidWork};
use lib3h_tracing::Lib3hSpan;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;

/// Network layer related private methods
impl<'engine> GhostEngine<'engine> {
    /// Process whatever the network has in for us.
    pub(crate) fn process_multiplexer(&mut self) -> Lib3hResult<DidWork> {
        // Process the network gateway
        detach_run!(&mut self.multiplexer, |ng| ng.process(self))?;
        for mut request in self.multiplexer.drain_messages() {
            let payload = request.take_message().expect("exists");
            match payload {
                GatewayRequestToParent::Dht(dht_request) => {
                    self.handle_network_dht_request(dht_request)?;
                }
                GatewayRequestToParent::Transport(transport_request) => {
                    self.handle_network_transport_request(&transport_request)?;
                }
            }
        }
        // Done
        Ok(true /* fixme */)
    }

    /// Handle a DhtRequestToParent sent to us by our network gateway
    fn handle_network_dht_request(&mut self, request: DhtRequestToParent) -> Lib3hResult<()> {
        debug!("{} << handle_network_dht_request: {:?}", self.name, request);
        match request {
            DhtRequestToParent::GossipTo(gossip_data) => {
                handle_gossip_to(NETWORK_GATEWAY_ID, self.multiplexer.as_mut(), gossip_data)
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
                    self.name, peer_data.peer_address, peer_data.peer_uri,
                );
                let cmd = GatewayRequestToChild::Transport(
                    transport::protocol::RequestToChild::SendMessage {
                        uri: peer_data.peer_uri,
                        payload: Opaque::new(),
                    },
                );
                self.multiplexer.publish(Lib3hSpan::todo(), cmd)?;
            }
            DhtRequestToParent::PeerTimedOut(_peer_address) => {
                // Disconnect from that peer by calling a Close on it.
                // FIXME
            }
            // No entries in Network DHT
            DhtRequestToParent::HoldEntryRequested {
                from_peer: _,
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
    fn handle_network_transport_request(
        &mut self,
        request: &transport::protocol::RequestToParent,
    ) -> Lib3hResult<()> {
        debug!(
            "{} << handle_network_transport_request: {:?}",
            self.name, request
        );
        // Note: use same order as the enum
        match request {
            transport::protocol::RequestToParent::ErrorOccured { uri, error } => {
                self.network_connections.remove(uri);
                error!("{} Network error from {} : {:?}", self.name, uri, error);
                // Output a Lib3hToClient::Disconnected if it was the connection
                if self.network_connections.is_empty() {
                    let data = DisconnectedData {
                        network_id: "FIXME".to_string(), // TODO #172
                    };
                    self.lib3h_endpoint
                        .publish(Lib3hSpan::todo(), Lib3hToClient::Disconnected(data))?;
                }
            }
            transport::protocol::RequestToParent::IncomingConnection { uri } => {
                self.handle_incoming_connection(uri.clone())?;
            }
            //            TransportEvent::ConnectionClosed(id) => {
            //                self.network_connections.remove(id);
            //                // Output a Lib3hToClient::Disconnected if it was the last connection
            //                if self.network_connections.is_empty() {
            //                    let data = DisconnectedData {
            //                        network_id: "FIXME".to_string(), // TODO #172
            //                    };
            //                    outbox.push(Lib3hToClient::Disconnected(data));
            //                }
            //            }
            transport::protocol::RequestToParent::ReceivedData { uri, payload } => {
                debug!("Received message from: {} | size: {}", uri, payload.len());
                let mut de = Deserializer::new(&payload[..]);
                let maybe_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                if let Err(e) = maybe_msg {
                    error!("Failed deserializing msg: {:?}", e);
                    return Err(Lib3hError::new(ErrorKind::RmpSerdeDecodeError(e)));
                }
                let p2p_msg = maybe_msg.unwrap();
                self.serve_P2pProtocol(uri, &p2p_msg)?;
            }
        };
        Ok(())
    }

    fn handle_incoming_connection(&mut self, net_uri: Url) -> Lib3hResult<()> {
        // Get list of known peers
        let uri_copy = net_uri.clone();
        self.multiplexer
            .request(
                Lib3hSpan::todo(),
                GatewayRequestToChild::Dht(DhtRequestToChild::RequestPeerList),
                Box::new(move |me, response| {
                    let response = {
                        match response {
                            GhostCallbackData::Timeout => panic!("timeout"),
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
                                uri_copy,
                            );
                            // we need a transportId, so search for it in the DHT
                            let maybe_peer_data =
                                peer_list.iter().find(|pd| pd.peer_uri == uri_copy);
                            if let Some(peer_data) = maybe_peer_data {
                                trace!("AllJoinedSpaceList ; sending back to {:?}", peer_data);
                                /* TODO: #777
                                me.multiplexer.publish(
                                    Lib3hSpan::todo(),
                                    GatewayRequestToChild::Transport(
                                        transport::protocol::RequestToChild::SendMessage {
                                            uri: Url::parse(&peer_data.peer_address)
                                                .expect("invalid url format"),
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
                uri: net_uri.clone(),
            };
            self.lib3h_endpoint
                .publish(Lib3hSpan::todo(), Lib3hToClient::Connected(data))?;
        }
        let _ = self.network_connections.insert(net_uri.to_owned());
        Ok(())
    }

    /// Serve a P2pProtocol sent to us by the network.
    /// TODO #150
    #[allow(non_snake_case)]
    fn serve_P2pProtocol(&mut self, _from: &Url, p2p_msg: &P2pProtocol) -> Lib3hResult<()> {
        match p2p_msg {
            P2pProtocol::Gossip(msg) => {
                // Prepare remoteGossipTo to post to dht
                let gossip = RemoteGossipBundleData {
                    from_peer_address: msg.from_peer_address.clone().into(),
                    bundle: msg.bundle.clone(),
                };
                // Check if its for the multiplexer
                if msg.space_address.to_string() == NETWORK_GATEWAY_ID {
                    let _ = self.multiplexer.publish(
                        Lib3hSpan::todo(),
                        GatewayRequestToChild::Dht(DhtRequestToChild::HandleGossip(gossip)),
                    );
                } else {
                    // otherwise should be for one of our space
                    let maybe_space_gateway = self
                        .space_gateway_map
                        .get_mut(&(msg.space_address.to_owned(), msg.to_peer_address.to_owned()));
                    if let Some(space_gateway) = maybe_space_gateway {
                        let _ = space_gateway.publish(
                            Lib3hSpan::todo(),
                            GatewayRequestToChild::Dht(DhtRequestToChild::HandleGossip(gossip)),
                        );
                    } else {
                        warn!("received gossip for unjoined space: {}", msg.space_address);
                    }
                }
            }
            P2pProtocol::DirectMessage(dm_data) => {
                let maybe_space_gateway = self.space_gateway_map.get(&(
                    dm_data.space_address.to_owned(),
                    dm_data.to_agent_id.to_owned(),
                ));
                if let Some(_) = maybe_space_gateway {
                    // Change into Lib3hToClient
                    let lib3_msg = Lib3hToClient::HandleSendDirectMessage(dm_data.clone());
                    self.lib3h_endpoint.publish(Lib3hSpan::todo(), lib3_msg)?;
                } else {
                    warn!(
                        "Received message from unjoined space: {}",
                        dm_data.space_address,
                    );
                }
            }
            P2pProtocol::DirectMessageResult(dm_data) => {
                let maybe_space_gateway = self.space_gateway_map.get(&(
                    dm_data.space_address.to_owned(),
                    dm_data.to_agent_id.to_owned(),
                ));
                if let Some(_) = maybe_space_gateway {
                    let lib3_msg = Lib3hToClient::SendDirectMessageResult(dm_data.clone());
                    self.lib3h_endpoint.publish(Lib3hSpan::todo(), lib3_msg)?;
                } else {
                    warn!(
                        "Received message from unjoined space: {}",
                        dm_data.space_address,
                    );
                }
            }
            P2pProtocol::PeerAddress(_, _, _) => {
                // no-op
            }
            P2pProtocol::BroadcastJoinSpace(gateway_id, peer_data) => {
                debug!("Received JoinSpace: {} {:?}", gateway_id, peer_data);
                for (_, space_gateway) in self.space_gateway_map.iter_mut() {
                    space_gateway.publish(
                        Lib3hSpan::todo(),
                        GatewayRequestToChild::Dht(DhtRequestToChild::HoldPeer(peer_data.clone())),
                    )?;
                }
            }
            P2pProtocol::AllJoinedSpaceList(join_list) => {
                debug!("Received AllJoinedSpaceList: {:?}", join_list);
                for (space_address, peer_data) in join_list {
                    let maybe_space_gateway = self.get_first_space_mut(space_address);
                    if let Some(space_gateway) = maybe_space_gateway {
                        let _ = space_gateway.publish(
                            Lib3hSpan::todo(),
                            GatewayRequestToChild::Dht(DhtRequestToChild::HoldPeer(
                                peer_data.clone(),
                            )),
                        )?;
                    }
                }
            }
        };
        Ok(())
    }
}
