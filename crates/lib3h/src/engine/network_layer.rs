#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::{p2p_protocol::P2pProtocol, RealEngine, NETWORK_GATEWAY_ID},
    transport::{protocol::*, transport_trait::Transport, ConnectionIdRef},
};
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol, DidWork, Lib3hResult};

use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

/// Network layer realted private methods
impl<T: Transport, D: Dht> RealEngine<T, D> {
    /// Process whatever the network has in for us.
    pub(crate) fn process_network_gateway(
        &mut self,
    ) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        let mut outbox = Vec::new();
        // Process the network gateway as a transport
        let (tranport_did_work, event_list) =
            Transport::process(&mut *self.network_gateway.borrow_mut())?;
        debug!(
            "{} - network_gateway Transport.process(): {} {}",
            self.name.clone(),
            tranport_did_work,
            event_list.len()
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
        debug!("{} << handle_netDhtEvent: {:?}", self.name.clone(), cmd);
        let outbox = Vec::new();
        match cmd {
            DhtEvent::GossipTo(_data) => {
                // FIXME
            }
            DhtEvent::GossipUnreliablyTo(_data) => {
                // FIXME
            }
            DhtEvent::HoldPeerRequested(peer_data) => {
                // #fullsync HACK
                // Connect to every peer
                info!(
                    "{} auto-connect to peer: {} ({})",
                    self.name.clone(),
                    peer_data.peer_address,
                    peer_data.peer_uri
                );
                let cmd = TransportCommand::Connect(peer_data.peer_uri.clone());
                Transport::post(&mut *self.network_gateway.borrow_mut(), cmd)?;
            }
            DhtEvent::PeerTimedOut(_data) => {
                // FIXME
            }
            DhtEvent::HoldEntryRequested(_from, _entry) => {
                // FIXME
            }
            DhtEvent::FetchEntryResponse(_data) => {
                // FIXME
            }
            DhtEvent::EntryPruned(_address) => {
                // FIXME
            }
            DhtEvent::EntryDataRequested(_) => {
                // FIXME
            }
        }
        Ok(outbox)
    }

    /// Handle a TransportEvent sent to us by our network gateway
    fn handle_netTransportEvent(
        &mut self,
        evt: &TransportEvent,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        debug!(
            "{} << handle_netTransportEvent: {:?}",
            self.name.clone(),
            evt
        );
        let mut outbox = Vec::new();
        // Note: use same order as the enum
        match evt {
            TransportEvent::TransportError(id, e) => {
                self.network_connections.remove(id);
                error!("{} Network error from {} : {:?}", self.name.clone(), id, e);
                // Output a Lib3hServerProtocol::Disconnected if it was the connection
                if self.network_connections.is_empty() {
                    let data = DisconnectedData {
                        network_id: "FIXME".to_string(),
                    };
                    outbox.push(Lib3hServerProtocol::Disconnected(data));
                }
            }
            TransportEvent::ConnectResult(id) => {
                let mut network_gateway = self.network_gateway.borrow_mut();
                if let Some(uri) = network_gateway.get_uri(id) {
                    info!("Network Connection opened: {} ({})", id, uri);

                    // FIXME: Do this in next process instead
                    // Send to other node our Joined Spaces
                    let space_list = self.get_all_spaces();
                    let our_joined_space_list = P2pProtocol::AllJoinedSpaceList(space_list);
                    let mut buf = Vec::new();
                    our_joined_space_list
                        .serialize(&mut Serializer::new(&mut buf))
                        .unwrap();
                    trace!(
                        "(GatewayTransport) P2pProtocol::AllJoinedSpaceList: {:?} to {:?}",
                        our_joined_space_list,
                        id
                    );
                    // id is connectionId but we need a transportId, so search for it in the DHT
                    let peer_list = network_gateway.get_peer_list();
                    trace!(
                        "(GatewayTransport) P2pProtocol::AllJoinedSpaceList: get_peer_list = {:?}",
                        peer_list
                    );
                    let maybe_peer_data = peer_list.iter().find(|pd| pd.peer_uri == uri);
                    if let Some(peer_data) = maybe_peer_data {
                        trace!(
                            "(GatewayTransport) P2pProtocol::AllJoinedSpaceList ; sending back to {:?}",
                            peer_data,
                        );
                        network_gateway.send(&[&peer_data.peer_address], &buf)?;
                    }
                    // FIXME END

                    // Output a Lib3hServerProtocol::Connected if its the first connection
                    if self.network_connections.is_empty() {
                        let data = ConnectedData {
                            request_id: "FIXME".to_string(),
                            uri,
                        };
                        outbox.push(Lib3hServerProtocol::Connected(data));
                    }
                    let _ = self.network_connections.insert(id.to_owned());
                }
            }
            TransportEvent::Connection(_id) => {
                // TODO!!!
                unimplemented!();
            }
            TransportEvent::Closed(id) => {
                self.network_connections.remove(id);
                // Output a Lib3hServerProtocol::Disconnected if it was the connection
                if self.network_connections.is_empty() {
                    let data = DisconnectedData {
                        network_id: "FIXME".to_string(),
                    };
                    outbox.push(Lib3hServerProtocol::Disconnected(data));
                }
            }
            TransportEvent::Received(id, payload) => {
                debug!("Received message from: {} | {}", id, payload.len());
                let mut de = Deserializer::new(&payload[..]);
                let maybe_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                if let Err(e) = maybe_msg {
                    return Err(format_err!("Failed deserializing msg: {:?}", e));
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
                    self.network_gateway.borrow_mut().post_dht(cmd)?;
                //Dht::post(&mut self.network_gateway.borrow_mut(), cmd);
                } else {
                    // otherwise should be for one of our space
                    let space_gateway = self
                        .space_gateway_map
                        .get_mut(&(msg.space_address.to_owned(), msg.to_peer_address.to_owned()))
                        .ok_or_else(|| {
                            format_err!("space_gateway not found: {}", msg.space_address)
                        })?;
                    space_gateway.post_dht(cmd)?;
                    // Dht::post(&mut space_gateway, cmd);
                }
            }
            P2pProtocol::DirectMessage(dm_data) => {
                let maybe_space_gateway = self.space_gateway_map.get(&(
                    dm_data.space_address.to_owned(),
                    dm_data.to_agent_id.to_owned(),
                ));
                //
                if let Some(_space_gateway) = maybe_space_gateway {
                    // TODO: check if we know sender?
                    //  let sender_address = std::string::String::from_utf8_lossy(&dm_data.from_agent_id).to_string();
                    //  let maybe_sender = _space_gateway.get_peer(sender_address);
                    // Change into Lib3hServerProtocol
                    let lib3_msg = Lib3hServerProtocol::HandleSendDirectMessage(dm_data.clone());
                    outbox.push(lib3_msg);
                } else {
                    warn!("Received message from unjoined space");
                }
            }
            P2pProtocol::DirectMessageResult(dm_data) => {
                let maybe_space_gateway = self.space_gateway_map.get(&(
                    dm_data.space_address.to_owned(),
                    dm_data.to_agent_id.to_owned(),
                ));
                //
                if let Some(_space_gateway) = maybe_space_gateway {
                    // TODO: check if we know sender?
                    //  let sender_address = std::string::String::from_utf8_lossy(&dm_data.from_agent_id).to_string();
                    //  let maybe_sender = _space_gateway.get_peer(sender_address);
                    // Change into Lib3hServerProtocol
                    let lib3_msg = Lib3hServerProtocol::SendDirectMessageResult(dm_data.clone());
                    outbox.push(lib3_msg);
                } else {
                    warn!("Received message from unjoined space");
                }
            }
            P2pProtocol::FetchData => {
                // FIXME
            }
            P2pProtocol::FetchDataResponse => {
                // FIXME
            }
            P2pProtocol::PeerAddress(_, _) => {
                // FIXME
            }
            // HACK
            P2pProtocol::BroadcastJoinSpace(gateway_id, peer_data) => {
                debug!("Received JoinSpace: {} {:?}", gateway_id, peer_data);
                for (_, space_gateway) in self.space_gateway_map.iter_mut() {
                    space_gateway.post_dht(DhtCommand::HoldPeer(peer_data.clone()))?;
                }
            }
            // HACK
            P2pProtocol::AllJoinedSpaceList(join_list) => {
                debug!("Received AllJoinedSpaceList: {:?}", join_list);
                for (space_address, peer_data) in join_list {
                    let maybe_space_gateway = self.get_first_space_mut(space_address);
                    if let Some(space_gateway) = maybe_space_gateway {
                        space_gateway.post_dht(DhtCommand::HoldPeer(peer_data.clone()))?;
                    }
                }
            }
        };
        Ok(outbox)
    }
}
