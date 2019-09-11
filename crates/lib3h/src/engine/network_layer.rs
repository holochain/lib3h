#![allow(non_snake_case)]

use crate::{
    dht::dht_protocol::*,
    engine::{
        p2p_protocol::P2pProtocol, real_engine::handle_gossipTo, RealEngine, NETWORK_GATEWAY_ID,
    },
    error::{ErrorKind, Lib3hError, Lib3hResult},
    gateway::protocol::*,
    transport,
};

use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol, DidWork};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;

/// Network layer related private methods
impl RealEngine {
    /// Process whatever the network has in for us.
    pub(crate) fn process_network_gateway(
        &mut self,
    ) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        let mut outbox = Vec::new();
        // Process the network gateway
        self.network_gateway.process(&mut self.gateway_user_data)?;
        for mut request in self.network_gateway.drain_messages() {
            let payload = request.take_message().expect("exists");
            let mut output = self.handle_network_request(payload)?;
            outbox.append(&mut output);
        }
        //        // FIXME: DHT magic
        //        let mut temp = self.network_gateway.drain_dht_outbox();
        //        self.temp_outbox.append(&mut temp);
        // Done
        Ok((true /* fixme */, outbox))
    }

    /// Handle a GatewayRequestToParent sent to us by our network gateway
    fn handle_network_request(
        &mut self,
        request: GatewayRequestToParent,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        debug!("{} << handle_network_request: [{:?}]", self.name, request,);
        //let parent_request = request.clone();
        match request {
            GatewayRequestToParent::Dht(dht_request) => {
                self.handle_network_dht_request(dht_request)
            }
            GatewayRequestToParent::Transport(transport_request) => {
                self.handle_network_transport_request(&transport_request)
            }
        }
    }

    /// Handle a DhtRequestToParent sent to us by our network gateway
    fn handle_network_dht_request(
        &mut self,
        request: DhtRequestToParent,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        debug!("{} << handle_network_dht_request: {:?}", self.name, request);
        let outbox = Vec::new();
        match request {
            DhtRequestToParent::GossipTo(gossip_data) => {
                handle_gossipTo(NETWORK_GATEWAY_ID, &mut self.network_gateway, gossip_data)
                    .expect("Failed to gossip with network_gateway");
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
                        payload: Vec::new(),
                    },
                );
                self.network_gateway.publish(cmd)?;
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
        Ok(outbox)
    }

    /// Handle a TransportRequestToParent sent to us by our network gateway
    fn handle_network_transport_request(
        &mut self,
        request: &transport::protocol::RequestToParent,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        debug!(
            "{} << handle_network_transport_request: {:?}",
            self.name, request
        );
        let mut outbox = Vec::new();
        // Note: use same order as the enum
        match request {
            transport::protocol::RequestToParent::ErrorOccured { uri, error } => {
                self.network_connections.remove(uri);
                error!("{} Network error from {} : {:?}", self.name, uri, error);
                // Output a Lib3hServerProtocol::Disconnected if it was the connection
                if self.network_connections.is_empty() {
                    let data = DisconnectedData {
                        network_id: "FIXME".to_string(), // TODO #172
                    };
                    outbox.push(Lib3hServerProtocol::Disconnected(data));
                }
            }
            transport::protocol::RequestToParent::IncomingConnection { uri } => {
                let mut output = self.handle_incoming_connection(uri.clone())?;
                outbox.append(&mut output);
            }
            //            TransportEvent::ConnectionClosed(id) => {
            //                self.network_connections.remove(id);
            //                // Output a Lib3hServerProtocol::Disconnected if it was the last connection
            //                if self.network_connections.is_empty() {
            //                    let data = DisconnectedData {
            //                        network_id: "FIXME".to_string(), // TODO #172
            //                    };
            //                    outbox.push(Lib3hServerProtocol::Disconnected(data));
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
                let mut output = self.serve_P2pProtocol(uri, &p2p_msg)?;
                outbox.append(&mut output);
            }
        };
        Ok(outbox)
    }

    fn handle_incoming_connection(
        &mut self,
        net_uri: Url,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        // TODO #150 - Should do this in next process instead
        let peer_list = self.get_peer_list_sync();
        trace!("AllJoinedSpaceList: get_peer_list = {:?}", peer_list);

        // TODO #150 - Should do this in next process instead
        // Send to other node our Joined Spaces
        {
            let space_list = self.get_all_spaces();
            let our_joined_space_list = P2pProtocol::AllJoinedSpaceList(space_list);
            let mut payload = Vec::new();
            our_joined_space_list
                .serialize(&mut Serializer::new(&mut payload))
                .unwrap();
            trace!(
                "AllJoinedSpaceList: {:?} to {:?}",
                our_joined_space_list,
                net_uri,
            );
            // we need a transportId, so search for it in the DHT
            let maybe_peer_data = peer_list.iter().find(|pd| pd.peer_uri == net_uri);
            if let Some(peer_data) = maybe_peer_data {
                trace!("AllJoinedSpaceList ; sending back to {:?}", peer_data);
                self.network_gateway
                    .publish(GatewayRequestToChild::Transport(
                        transport::protocol::RequestToChild::SendMessage {
                            uri: Url::parse(&peer_data.peer_address).expect("invalid url format"),
                            payload,
                        },
                    ))?;
            }
            // TODO END
        }

        // Output a Lib3hServerProtocol::Connected if its the first connection
        let mut outbox = Vec::new();
        if self.network_connections.is_empty() {
            let data = ConnectedData {
                request_id: "fixme".to_string(),
                uri: net_uri.clone(),
            };
            outbox.push(Lib3hServerProtocol::Connected(data));
        }
        let _ = self.network_connections.insert(net_uri.to_owned());
        Ok(outbox)
    }

    /// Serve a P2pProtocol sent to us by the network.
    /// Return a list of Lib3hServerProtocol to send to Core.
    /// TODO #150
    fn serve_P2pProtocol(
        &mut self,
        _from: &Url,
        p2p_msg: &P2pProtocol,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        let mut outbox = Vec::new();
        match p2p_msg {
            P2pProtocol::Gossip(msg) => {
                // Prepare remoteGossipTo to post to dht
                let gossip = RemoteGossipBundleData {
                    from_peer_address: msg.from_peer_address.clone().into(),
                    bundle: msg.bundle.clone(),
                };
                // Check if its for the network_gateway
                if msg.space_address.to_string() == NETWORK_GATEWAY_ID {
                    let _ = self.network_gateway.publish(GatewayRequestToChild::Dht(
                        DhtRequestToChild::HandleGossip(gossip),
                    ));
                } else {
                    // otherwise should be for one of our space
                    let maybe_space_gateway = self
                        .space_gateway_map
                        .get_mut(&(msg.space_address.to_owned(), msg.to_peer_address.to_owned()));
                    if let Some(space_gateway) = maybe_space_gateway {
                        let _ = space_gateway.publish(GatewayRequestToChild::Dht(
                            DhtRequestToChild::HandleGossip(gossip),
                        ));
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
                    // Change into Lib3hServerProtocol
                    let lib3_msg = Lib3hServerProtocol::HandleSendDirectMessage(dm_data.clone());
                    outbox.push(lib3_msg);
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
                    let lib3_msg = Lib3hServerProtocol::SendDirectMessageResult(dm_data.clone());
                    outbox.push(lib3_msg);
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
                    let _ = space_gateway.publish(GatewayRequestToChild::Dht(
                        DhtRequestToChild::HoldPeer(peer_data.clone()),
                    ));
                }
            }
            P2pProtocol::AllJoinedSpaceList(join_list) => {
                debug!("Received AllJoinedSpaceList: {:?}", join_list);
                for (space_address, peer_data) in join_list {
                    let maybe_space_gateway = self.get_first_space_mut(space_address);
                    if let Some(space_gateway) = maybe_space_gateway {
                        let _ = space_gateway.publish(GatewayRequestToChild::Dht(
                            DhtRequestToChild::HoldPeer(peer_data.clone()),
                        ));
                    }
                }
            }
        };
        Ok(outbox)
    }
}
