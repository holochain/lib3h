#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::{p2p_protocol::P2pProtocol, RealEngine, NETWORK_GATEWAY_ID},
    error::{ErrorKind, Lib3hError, Lib3hResult},
    transport::{protocol::*, transport_trait::Transport, ConnectionIdRef},
};
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol, DidWork};

use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

/// Network layer related private methods
impl<T: Transport, D: Dht> RealEngine<T, D> {
    /// Process whatever the network has in for us.
    pub(crate) fn process_network_gateway(
        &mut self,
    ) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        let mut outbox = Vec::new();
        // Process the network gateway as a Transport
        let (tranport_did_work, event_list) =
            Transport::process(&mut *self.network_gateway.borrow_mut())?;
        debug!(
            "{} - network_gateway Transport.process(): {} {}",
            self.name,
            tranport_did_work,
            event_list.len(),
        );
        if tranport_did_work {
            for evt in event_list {
                let mut output = self.handle_netTransportEvent(&evt)?;
                outbox.append(&mut output);
            }
        }
        // Process the network gateway as a DHT
        let (dht_did_work, event_list) = Dht::process(&mut *self.network_gateway.borrow_mut())?;
        if dht_did_work {
            for evt in event_list {
                let mut output = self.handle_netDhtEvent(evt)?;
                outbox.append(&mut output);
            }
        }
        Ok((tranport_did_work || dht_did_work, outbox))
    }

    /// Handle a DhtEvent sent to us by our network gateway
    fn handle_netDhtEvent(&mut self, cmd: DhtEvent) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        debug!("{} << handle_netDhtEvent: {:?}", self.name, cmd);
        let outbox = Vec::new();
        match cmd {
            DhtEvent::GossipTo(_data) => {
                // no-op
            }
            DhtEvent::GossipUnreliablyTo(_data) => {
                // no-op
            }
            DhtEvent::HoldPeerRequested(peer_data) => {
                // TODO #167 - hardcoded for MirrorDHT and thus should not appear here.
                // Connect to every peer we are requested to hold.
                info!(
                    "{} auto-connect to peer: {} ({})",
                    self.name, peer_data.peer_address, peer_data.peer_uri,
                );
                let cmd = TransportCommand::Connect(peer_data.peer_uri.clone(), "".to_string());
                Transport::post(&mut *self.network_gateway.borrow_mut(), cmd)?;
            }
            DhtEvent::PeerTimedOut(peer_address) => {
                // Disconnect from that peer by calling a Close on it.
                let mut network_gateway = self.network_gateway.borrow_mut();
                let maybe_connection_id = network_gateway.get_connection_id(&peer_address);
                trace!(
                    "{} -- maybe_connection_id: {:?}",
                    self.name.clone(),
                    maybe_connection_id,
                );
                if let Some(connection_id) = maybe_connection_id {
                    Transport::post(
                        &mut *network_gateway,
                        TransportCommand::Close(connection_id),
                    )?;
                }
            }
            // No entries in Network DHT
            DhtEvent::HoldEntryRequested(_, _) => {
                unreachable!();
            }
            DhtEvent::FetchEntryResponse(_) => {
                unreachable!();
            }
            DhtEvent::EntryPruned(_) => {
                unreachable!();
            }
            DhtEvent::EntryDataRequested(_) => {
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
        let mut outbox = Vec::new();
        let mut network_gateway = self.network_gateway.borrow_mut();
        if let Some(uri) = network_gateway.get_uri(id) {
            info!("Network Connection opened: {} ({})", id, uri);
            // TODO #150 - Should do this in next process instead
            // Send to other node our Joined Spaces
            let space_list = self.get_all_spaces();
            let our_joined_space_list = P2pProtocol::AllJoinedSpaceList(space_list);
            let mut buf = Vec::new();
            our_joined_space_list
                .serialize(&mut Serializer::new(&mut buf))
                .unwrap();
            trace!(
                "AllJoinedSpaceList: {:?} to {:?}",
                our_joined_space_list,
                id
            );
            // id is connectionId but we need a transportId, so search for it in the DHT
            let peer_list = network_gateway.get_peer_list();
            trace!("AllJoinedSpaceList: get_peer_list = {:?}", peer_list);
            let maybe_peer_data = peer_list.iter().find(|pd| pd.peer_uri == uri);
            if let Some(peer_data) = maybe_peer_data {
                trace!("AllJoinedSpaceList ; sending back to {:?}", peer_data);
                network_gateway.send(&[&peer_data.peer_address], &buf)?;
            }
            // TODO END

            // Output a Lib3hServerProtocol::Connected if its the first connection
            if self.network_connections.is_empty() {
                let data = ConnectedData { request_id, uri };
                outbox.push(Lib3hServerProtocol::Connected(data));
            }
            let _ = self.network_connections.insert(id.to_owned());
        }
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
            TransportEvent::SuccessResult(msg) => {
                debug!("SUCCESS RESULT: {:?}", msg);
            }
            TransportEvent::FailureResult(msg) => {
                error!("FAILURE RESULT: {:?}", msg);
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
                let cmd = DhtCommand::HandleGossip(RemoteGossipBundleData {
                    from_peer_address: msg.from_peer_address.clone().into(),
                    bundle: msg.bundle.clone(),
                });
                // Check if its for the network_gateway
                if msg.space_address.to_string() == NETWORK_GATEWAY_ID {
                    Dht::post(&mut *self.network_gateway.borrow_mut(), cmd)?;
                } else {
                    // otherwise should be for one of our space
                    let maybe_space_gateway = self
                        .space_gateway_map
                        .get_mut(&(msg.space_address.to_owned(), msg.to_peer_address.to_owned()));
                    if let Some(space_gateway) = maybe_space_gateway {
                        Dht::post(space_gateway, cmd)?;
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
                    Dht::post(space_gateway, DhtCommand::HoldPeer(peer_data.clone()))?;
                }
            }
            P2pProtocol::AllJoinedSpaceList(join_list) => {
                debug!("Received AllJoinedSpaceList: {:?}", join_list);
                for (space_address, peer_data) in join_list {
                    let maybe_space_gateway = self.get_first_space_mut(space_address);
                    if let Some(space_gateway) = maybe_space_gateway {
                        Dht::post(space_gateway, DhtCommand::HoldPeer(peer_data.clone()))?;
                    }
                }
            }
        };
        Ok(outbox)
    }
}
