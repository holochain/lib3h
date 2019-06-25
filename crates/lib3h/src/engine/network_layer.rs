#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::{p2p_protocol::P2pProtocol, RealEngine},
    transport::{protocol::*, transport_trait::Transport, TransportIdRef},
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
        println!(
            "[d] {} - network_gateway Transport.process(): {} {}",
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
        println!("[d] {} << handle_netDhtEvent: {:?}", self.name.clone(), cmd);
        let outbox = Vec::new();
        match cmd {
            DhtEvent::GossipTo(_data) => {
                // FIXME
            }
            DhtEvent::GossipUnreliablyTo(_data) => {
                // FIXME
            }
            DhtEvent::HoldPeerRequested(_peer_address) => {
                // FIXME
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
        }
        Ok(outbox)
    }

    /// Handle a TransportEvent sent to us by our network gateway
    fn handle_netTransportEvent(
        &mut self,
        evt: &TransportEvent,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        println!(
            "[d] {} << handle_netTransportEvent: {:?}",
            self.name.clone(),
            evt
        );
        let mut outbox = Vec::new();
        // Note: use same order as the enum
        match evt {
            TransportEvent::TransportError(id, e) => {
                self.network_connections.remove(id);
                eprintln!(
                    "[e] {} Network error from {} : {:?}",
                    self.name.clone(),
                    id,
                    e
                );
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
                    println!("[i] Network Connection opened: {} ({})", id, uri);

                    // Send to other node our Joined Spaces
                    let space_list = self.get_all_spaces();
                    let our_joined_space_list = P2pProtocol::AllJoinedSpaceList(space_list);
                    let mut buf = Vec::new();
                    our_joined_space_list
                        .serialize(&mut Serializer::new(&mut buf))
                        .unwrap();
                    println!(
                        "(GatewayTransport) P2pProtocol::AllJoinedSpaceList: {:?} to {:?}",
                        our_joined_space_list, id
                    );
                    // id is connection_id but we need a machine_id, so search for it in the DHT
                    let peer_list = network_gateway.get_peer_list();
                    let maybe_peer_data = peer_list.iter().find(|pd| pd.transport == uri);
                    if let Some(peer_data) = maybe_peer_data {
                        println!(
                            "(GatewayTransport) P2pProtocol::AllJoinedSpaceList ; sending back to {:?}",
                            peer_data,
                        );
                        network_gateway.send(&[&peer_data.peer_address], &buf)?;
                    }

                    // Output a Lib3hServerProtocol::Connected if its the first connection
                    if self.network_connections.is_empty() {
                        let data = ConnectedData {
                            request_id: "FIXME".to_string(),
                            network_transport: uri.to_string(),
                        };
                        outbox.push(Lib3hServerProtocol::Connected(data));
                    }
                    let _ = self.network_connections.insert(id.to_owned());
                }
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
                println!("[d] Received message from: {} | {}", id, payload.len());
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
        _from_id: &TransportIdRef,
        p2p_msg: &P2pProtocol,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        let mut outbox = Vec::new();
        match p2p_msg {
            P2pProtocol::Gossip(msg) => {
                let space_gateway = self
                    .space_gateway_map
                    .get_mut(&(msg.space_address.to_owned(), msg.to_peer_address.to_owned()))
                    .ok_or_else(|| format_err!("space_gateway not found"))?;
                // Post it as a remoteGossipTo
                let from_peer_address =
                    std::string::String::from_utf8_lossy(&msg.from_peer_address).into_owned();
                let cmd = DhtCommand::HandleGossip(RemoteGossipBundleData {
                    from_peer_address,
                    bundle: msg.bundle.clone(),
                });
                space_gateway.post_dht(cmd)?;
                // Dht::post(&mut space_gateway, cmd);
            }
            P2pProtocol::DirectMessage(data) => {
                // FIXME: check with space gateway first?
                // Change into Lib3hServerProtocol
                let lib3_msg = Lib3hServerProtocol::HandleSendDirectMessage(data.clone());
                outbox.push(lib3_msg);
            }
            P2pProtocol::DirectMessageResult(data) => {
                // FIXME: check with space gateway first?
                // Change into Lib3hServerProtocol
                let lib3_msg = Lib3hServerProtocol::SendDirectMessageResult(data.clone());
                outbox.push(lib3_msg);
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
                println!("[d] Received JoinSpace: {} {:?}", gateway_id, peer_data);
                for (_, space_gateway) in self.space_gateway_map.iter_mut() {
                    space_gateway.post_dht(DhtCommand::HoldPeer(peer_data.clone()))?;
                }
            }
            // HACK
            P2pProtocol::AllJoinedSpaceList(join_list) => {
                println!("[d] Received AllJoinedSpaceList: {:?}", join_list);
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
