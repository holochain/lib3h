#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, ghost_protocol::*},
    engine::{p2p_protocol::P2pProtocol, RealEngine, NETWORK_GATEWAY_ID},
    error::{ErrorKind, Lib3hError, Lib3hResult},
    transport::{protocol::*, ConnectionIdRef},
};
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol, DidWork};
use lib3h_ghost_actor::prelude::*;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

/// Network layer related private methods
impl<'engine> RealEngine<'engine> {
    /// Process whatever the network has in for us.
    pub(crate) fn process_network_gateway(
        &mut self,
    ) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        let mut did_work = false;
        let mut outbox = Vec::new();
        // Process the network gateway as a Transport
        let (tranport_did_work, event_list) = self.network_transport.as_mut().process()?;
        debug!(
            "{} - network_gateway Transport.process(): {} {}",
            self.name,
            tranport_did_work,
            event_list.len(),
        );
        if tranport_did_work {
            did_work = true;
        }
        for evt in &event_list {
            self.network_gateway
                .as_mut()
                .transport_inject_event(evt.clone());
        }
        {
            let (gateway_did_work, _event_list) =
                self.network_gateway.as_transport_mut().process()?;
            if gateway_did_work {
                did_work = true;
            }
        }
        for evt in event_list {
            let mut output = self.handle_netTransportEvent(&evt)?;
            outbox.append(&mut output);
        }
        // Process the network gateway as a DHT
        let dht_did_work = self.network_gateway.as_dht_mut().process(&mut ()).unwrap(); // FIXME
        for request in self.network_gateway.as_dht_mut().drain_messages() {
            self.handle_netDhtRequest(request)?;
        }
//        if bool::from(dht_did_work) {
//            did_work = true;
//        }
        Ok((did_work, outbox))
    }

    /// Handle a DhtEvent sent to us by our network gateway
    fn handle_netDhtRequest(&mut self, mut msg: DhtToParentMessage) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        debug!("{} << handle_netDhtEvent: {:?}", self.name, msg);
        let outbox = Vec::new();
        match msg.take_message().expect("exists") {
            DhtRequestToParent::GossipTo(_data) => {
                // no-op
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
                let cmd = TransportCommand::Connect(peer_data.peer_uri.clone(), "".to_string());
                self.network_gateway.as_transport_mut().post(cmd)?;
            }
            DhtRequestToParent::PeerTimedOut(peer_address) => {
                // Disconnect from that peer by calling a Close on it.
                let maybe_connection_id = self
                    .network_gateway
                    .as_ref()
                    .get_connection_id(&peer_address);
                trace!(
                    "{} -- maybe_connection_id: {:?}",
                    self.name.clone(),
                    maybe_connection_id,
                );
                if let Some(connection_id) = maybe_connection_id {
                    self.network_gateway
                        .as_transport_mut()
                        .post(TransportCommand::Close(connection_id))?;
                }
            }
            // No entries in Network DHT
            DhtRequestToParent::HoldEntryRequested {from_peer: _, entry: _} => {
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

    fn handle_new_connection(
        &mut self,
        id: &ConnectionIdRef,
        request_id: String,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        // get uri from id
        let mut network_transport = self.network_gateway.as_transport_mut();
        let maybe_uri = network_transport.get_uri(id);
        if maybe_uri.is_none() {
            return Ok(Vec::new());
        }
        let uri = maybe_uri.unwrap();
        info!("Network Connection opened: {} ({})", id, uri);
        // TODO #150 - Should do this in next process instead
        // Send to other node our Joined Spaces
        let space_list = self.get_all_spaces();
        let our_joined_space_list = P2pProtocol::AllJoinedSpaceList(space_list);
        let mut payload = Vec::new();
        our_joined_space_list
            .serialize(&mut Serializer::new(&mut payload))
            .unwrap();
        trace!(
            "AllJoinedSpaceList: {:?} to {:?}",
            our_joined_space_list,
            id
        );
        // id is connectionId but we need a transportId, so search for it in the DHT

        let peer_list = self.network_gateway.as_mut().get_peer_list_sync();
        trace!("AllJoinedSpaceList: get_peer_list = {:?}", peer_list);
        let maybe_peer_data = peer_list.iter().find(|pd| pd.peer_uri == uri);
        if let Some(peer_data) = maybe_peer_data {
            trace!("AllJoinedSpaceList ; sending back to {:?}", peer_data);
            network_transport.send(&[&peer_data.peer_address], &payload)?;
        }
        // TODO END

        // Output a Lib3hServerProtocol::Connected if its the first connection
        let mut outbox = Vec::new();
        if self.network_connections.is_empty() {
            let data = ConnectedData { request_id, uri };
            outbox.push(Lib3hServerProtocol::Connected(data));
        }
        let _ = self.network_connections.insert(id.to_owned());
        Ok(outbox)
    }

    /// Handle a TransportEvent sent to us by our network gateway
    fn handle_netTransportEvent(
        &mut self,
        evt: &TransportEvent,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        debug!("{} << handle_netTransportEvent: {:?}", self.name, evt);
        let mut outbox = Vec::new();
        // Note: use same order as the enum
        match evt {
            TransportEvent::ErrorOccured(id, e) => {
                self.network_connections.remove(id);
                error!("{} Network error from {} : {:?}", self.name, id, e);
                // Output a Lib3hServerProtocol::Disconnected if it was the connection
                if self.network_connections.is_empty() {
                    let data = DisconnectedData {
                        network_id: "FIXME".to_string(), // TODO #172
                    };
                    outbox.push(Lib3hServerProtocol::Disconnected(data));
                }
            }
            TransportEvent::ConnectResult(id, request_id) => {
                let mut output = self.handle_new_connection(id, request_id.clone())?;
                outbox.append(&mut output);
            }
            TransportEvent::IncomingConnectionEstablished(id) => {
                let mut output = self.handle_new_connection(id, "".to_string())?;
                outbox.append(&mut output);
            }
            TransportEvent::ConnectionClosed(id) => {
                self.network_connections.remove(id);
                // Output a Lib3hServerProtocol::Disconnected if it was the last connection
                if self.network_connections.is_empty() {
                    let data = DisconnectedData {
                        network_id: "FIXME".to_string(), // TODO #172
                    };
                    outbox.push(Lib3hServerProtocol::Disconnected(data));
                }
            }
            TransportEvent::ReceivedData(id, payload) => {
                debug!("Received message from: {} | {}", id, payload.len());
                let mut de = Deserializer::new(&payload[..]);
                let maybe_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                if let Err(e) = maybe_msg {
                    error!("Failed deserializing msg: {:?}", e);
                    return Err(Lib3hError::new(ErrorKind::RmpSerdeDecodeError(e)));
                }
                let p2p_msg = maybe_msg.unwrap();
                let mut output = self.serve_P2pProtocol(id, &p2p_msg)?;
                outbox.append(&mut output);
            }
        };
        Ok(outbox)
    }

    /// Serve a P2pProtocol sent to us by the network.
    /// Return a list of Lib3hServerProtocol to send to Core.
    /// TODO #150
    fn serve_P2pProtocol(
        &mut self,
        _from_id: &ConnectionIdRef,
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
                    self.network_gateway
                        .as_dht_mut()
                        .publish(DhtRequestToChild::HandleGossip(gossip));
                } else {
                    // otherwise should be for one of our space
                    let maybe_space_gateway = self
                        .space_gateway_map
                        .get_mut(&(msg.space_address.to_owned(), msg.to_peer_address.to_owned()));
                    if let Some(space_gateway) = maybe_space_gateway {
                        space_gateway
                            .as_dht_mut()
                            .publish(DhtRequestToChild::HandleGossip(gossip));
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
                    space_gateway
                        .as_dht_mut()
                        .publish(DhtRequestToChild::HoldPeer(peer_data.clone()));
                }
            }
            P2pProtocol::AllJoinedSpaceList(join_list) => {
                debug!("Received AllJoinedSpaceList: {:?}", join_list);
                for (space_address, peer_data) in join_list {
                    let maybe_space_gateway = self.get_first_space_mut(space_address);
                    if let Some(space_gateway) = maybe_space_gateway {
                        space_gateway
                            .as_dht_mut()
                            .publish(DhtRequestToChild::HoldPeer(peer_data.clone()));
                    }
                }
            }
        };
        Ok(outbox)
    }
}
