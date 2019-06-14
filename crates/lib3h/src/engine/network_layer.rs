#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::{p2p_protocol::P2pProtocol, real_engine::RealEngine, ChainId, RealEngineConfig},
    gateway::{
        gateway_dht::{self, *},
        gateway_transport,
        p2p_gateway::P2pGateway,
    },
    transport::{protocol::*, transport_trait::Transport},
};
use lib3h_protocol::{
    data_types::*, network_engine::NetworkEngine, protocol_client::Lib3hClientProtocol,
    protocol_server::Lib3hServerProtocol, Address, AddressRef, DidWork, Lib3hResult,
};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

/// Private
impl<'t, T: Transport, D: Dht> RealEngine<'t, T, D> {
    /// Process whatever the network has in for us.
    pub(crate) fn process_network_gateway(
        &mut self,
    ) -> Lib3hResult<(DidWork, Vec<Lib3hServerProtocol>)> {
        let mut outbox = Vec::new();
        // Process the network's transport
        let (tranport_did_work, event_list) = Transport::process(&mut self.network_gateway)?;
        if tranport_did_work {
            for evt in event_list {
                let mut output = self.handle_netTransportEvent(&evt)?;
                outbox.append(&mut output);
            }
        }
        // Process the network's DHT
        let (dht_did_work, mut event_list) = Dht::process(&mut self.network_gateway)?;
        if dht_did_work {
            for evt in event_list {
                let mut output = self.handle_netDhtEvent(evt)?;
                outbox.append(&mut output);
            }
        }
        Ok((tranport_did_work || dht_did_work, outbox))
    }

    /// Handle a DhtEvent sent to us by our internal DHT.
    fn handle_netDhtEvent(&mut self, cmd: DhtEvent) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        let mut outbox = Vec::new();
        match cmd {
            DhtEvent::GossipTo(data) => {
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

    ///
    fn handle_netTransportEvent(
        &mut self,
        evt: &TransportEvent,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        let mut outbox = Vec::new();
        // Note: use same order as the enum
        match evt {
            TransportEvent::TransportError(id, e) => {
                // FIXME
            }
            TransportEvent::ConnectResult(id) => {
                // FIXME
            }
            TransportEvent::Closed(id) => {
                // FIXME
            }
            TransportEvent::Received(id, payload) => {
                println!("(log.d) Received message from: {}", id);
                // FIXME: Make sense of msg? (i.e. deserialize)
                println!("Deserialize msg: {:?}", payload);
                let mut de = Deserializer::new(&payload[..]);
                let maybe_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                if let Err(e) = maybe_msg {
                    return Err(format_err!("Failed deserializing msg: {:?}", e));
                }
                let p2p_msg = maybe_msg.unwrap();
                let mut output = self.serve_P2pProtocol(&p2p_msg)?;
                outbox.append(&mut output);
            }
        };
        Ok(outbox)
    }

    /// Serve a P2pProtocol sent to us by the network.
    /// Return a list of Lib3hServerProtocol to send to Core.
    // FIXME
    fn serve_P2pProtocol(
        &mut self,
        p2p_msg: &P2pProtocol,
    ) -> Lib3hResult<Vec<Lib3hServerProtocol>> {
        let mut outbox = Vec::new();
        match p2p_msg {
            P2pProtocol::Gossip(msg) => {
                let mut space_gateway = self
                    .space_gateway_map
                    .get_mut(&(msg.space_address.to_owned(), msg.to_peer_address.to_owned()))
                    .ok_or(format_err!("space_gateway not found"))?;
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
        };
        Ok(outbox)
    }
}
