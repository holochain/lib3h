#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::{p2p_protocol::P2pProtocol, RealEngine, NETWORK_GATEWAY_ID},
    error::{ErrorKind, Lib3hError, Lib3hResult},
    transport::protocol::*,
};
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::*, protocol_server::Lib3hServerProtocol, DidWork};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;

/// Network layer related private methods
impl<'engine, D: Dht> RealEngine<'engine, D> {
    /// Process whatever the network has in for us.
    pub(crate) fn process_network_gateway(
        &mut self,
    ) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        // Process child network_gateway actor
        detach_run!(&mut self.network_gateway, |network_gateway| {
            network_gateway.process(&mut ())
        })
        .expect("network_gateway failed");
        // Act on child's requests
        let mut outbox = Vec::new();
        for mut request in self.network_gateway.drain_messages() {
            match request.take_message().expect("msg doesn't exist") {
                TransportRequestToParent::IncomingConnection { address } => {
                    // TODO
                    let mut output = self.handle_new_connection(&address, "".to_string())?;
                    outbox.append(&mut output);
                }
                TransportRequestToParent::ReceivedData { address, payload } => {
                    // TODO
                    debug!("Received message from: {} | {}", address, payload.len());
                    let mut de = Deserializer::new(&payload[..]);
                    let maybe_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                        Deserialize::deserialize(&mut de);
                    if let Err(e) = maybe_msg {
                        error!("Failed deserializing msg: {:?}", e);
                        return Err(Lib3hError::new(ErrorKind::RmpSerdeDecodeError(e)));
                    }
                    let p2p_msg = maybe_msg.unwrap();
                    let mut output = self.serve_P2pProtocol(&address, &p2p_msg)?;
                    outbox.append(&mut output);
                }
                TransportRequestToParent::ErrorOccured { address, error } => {
                    self.network_connections.remove(&address);
                    error!("{} Network error from {} : {:?}", self.name, address, error);
                    // Output a Lib3hServerProtocol::Disconnected if it was the last connection
                    if self.network_connections.is_empty() {
                        let data = DisconnectedData {
                            network_id: "FIXME".to_string(), // TODO #172
                        };
                        outbox.push(Lib3hServerProtocol::Disconnected(data));
                    }
                }
            };
        }
        // Done
        Ok((true, outbox))
    }

    pub(crate) fn network_connect(&mut self, connect_data: ConnectData) {
        // Send phony SendMessage request so we connect to it
        self.network_gateway
            .publish(TransportRequestToChild::SendMessage {
                address: connect_data.peer_uri,
                payload: Vec::new(),
            });
    }

    /// On new connection, send all joined spaces to other node
    fn handle_new_connection(
        &mut self,
        uri: &Url,
        request_id: String,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        let mut outbox = Vec::new();
        let mut network_gateway = self.network_gateway.as_mut();
        //if let Some(uri) = network_gateway.get_uri(uri) {
        info!("Network Connection opened to: {}", uri);
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
            uri
        );
        // We need a transportId, so search for it in the DHT
        //        let peer_list = network_gateway.get_peer_list();
        //        trace!("AllJoinedSpaceList: get_peer_list = {:?}", peer_list);
        //        let maybe_peer_data = peer_list.iter().find(|pd| pd.peer_uri == uri);
        //        if let Some(peer_data) = maybe_peer_data {
        //            trace!("AllJoinedSpaceList ; sending back to {:?}", peer_data);
        //            network_gateway.send(&[&peer_data.peer_address], &payload)?;
        //        }
        network_gateway.publish(TransportRequestToChild::SendMessage {
            address: uri.clone(),
            payload,
        });
        // TODO END

        // Output a Lib3hServerProtocol::Connected if its the first connection
        if self.network_connections.is_empty() {
            let data = ConnectedData {
                request_id,
                uri: uri.clone(),
            };
            outbox.push(Lib3hServerProtocol::Connected(data));
        }
        let _ = self.network_connections.insert(uri.to_owned());
        //}
        Ok(outbox)
    }

    /// Serve a P2pProtocol sent to us by the network.
    /// Return a list of Lib3hServerProtocol to send to Core.
    /// TODO #150
    fn serve_P2pProtocol(
        &mut self,
        _from_uri: &Url,
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
                // TODO Move this in GhostGateway ?
                //                if msg.space_address.to_string() == NETWORK_GATEWAY_ID {
                //                    // FIXME
                //                    self.network_gateway.as_dht_mut().post(cmd)?;
                //                } else {
                //                    // otherwise should be for one of our space
                //                    let maybe_space_gateway = self
                //                        .space_gateway_map
                //                        .get_mut(&(msg.space_address.to_owned(), msg.to_peer_address.to_owned()));
                //                    if let Some(space_gateway) = maybe_space_gateway {
                //                        space_gateway.as_dht_mut().post(cmd)?;
                //                    } else {
                //                        warn!("received gossip for unjoined space: {}", msg.space_address);
                //                    }
                //                }
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
                        .post(DhtCommand::HoldPeer(peer_data.clone()))?;
                }
            }
            P2pProtocol::AllJoinedSpaceList(join_list) => {
                debug!("Received AllJoinedSpaceList: {:?}", join_list);
                for (space_address, peer_data) in join_list {
                    let maybe_space_gateway = self.get_first_space_mut(space_address);
                    if let Some(space_gateway) = maybe_space_gateway {
                        space_gateway
                            .as_dht_mut()
                            .post(DhtCommand::HoldPeer(peer_data.clone()))?;
                    }
                }
            }
        };
        Ok(outbox)
    }
}
