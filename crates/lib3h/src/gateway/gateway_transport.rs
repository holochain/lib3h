#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::p2p_protocol::P2pProtocol,
    gateway::{Gateway, P2pGateway},
    transport::{
        error::{TransportError, TransportResult},
        protocol::*,
        transport_trait::Transport,
    },
};
use lib3h_protocol::DidWork;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;

/// Compose Transport
impl<'gateway, D: Dht> Transport for P2pGateway<'gateway, D> {
    ///
    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.transport_inbox.push_back(command);
        Ok(())
    }

    /// Handle TransportEvents directly
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Process TransportCommand inbox
        loop {
            let cmd = match self.transport_inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            let res = self.serve_TransportCommand(&cmd);
            if let Ok(mut output) = res {
                did_work = true;
                outbox.append(&mut output);
            }
        }
        trace!(
            "({}).Transport.process() - output: {} {}",
            self.identifier,
            did_work,
            outbox.len(),
        );
        // Process inner transport
        // Its okay to process inner transport as long as NetworkEngine only calls
        // Transport::process() on the network gateway,
        // otherwise remove this code and have RealEngine explicitly call the process of the
        // Network transport.
        let (inner_did_work, mut event_list) = self.inner_transport.as_mut().process()?;
        trace!(
            "({}).Transport.inner_process() - output: {} {}",
            self.identifier,
            inner_did_work,
            event_list.len()
        );
        if inner_did_work {
            did_work = true;
            outbox.append(&mut event_list);
        }
        // Handle TransportEvents
        for evt in outbox.clone().iter() {
            self.handle_TransportEvent(&evt)?;
        }
        Ok((did_work, outbox))
    }

    /// A Gateway uses its inner_dht's peerData.peer_address as connectionId
    fn connection_list(&self) -> TransportResult<Vec<Url>> {
        let peer_data_list = self.inner_dht.get_peer_list();
        let mut id_list = Vec::new();
        for peer_data in peer_data_list {
            id_list.push(Url::parse(&format!("hc:{}", peer_data.peer_address).expect("can parse url")));
        }
        Ok(id_list)
    }
}

/// Private internals
impl<'gateway, D: Dht> P2pGateway<'gateway, D> {
    ///
    fn priv_bind(&mut self, request_id: RequestId, spec: Url) -> TransportResult<()> {
        /*
        trace!("({}) bind() {}", self.identifier, url);
        self.inner_transport.as_mut().bind(url)
        */
        Ok(())
    }

    // TODO #176 - Return a higher-level uri instead?
    fn priv_connect(&mut self, request_id: RequestId, address: &Url) -> TransportResult<()> {
        /*
        trace!("({}).connect() {}", self.identifier, uri);
        // Connect
        let connection_id = self.inner_transport.as_mut().connect(&uri)?;
        // Store result in connection map
        self.connection_map
            .insert(uri.clone(), connection_id.clone());
        // Done
        Ok(connection_id)
        */
        Ok(())
    }

    /// id_list =
    ///   - Network : transportId
    ///   - space   : agentId
    fn priv_send(&mut self, request_id: RequestId, address: Url, payload: Vec<u8>) -> TransportResult<()> {
        /*
        // get connectionId from the inner dht first
        let dht_uri_list = self.dht_address_to_uri_list(dht_id_list)?;
        // send
        trace!(
            "({}).send() {:?} -> {:?} | {}",
            self.identifier,
            dht_id_list,
            dht_uri_list,
            payload.len(),
        );
        // Get connectionIds for the inner Transport.
        let mut conn_list = Vec::new();
        for dht_uri in dht_uri_list {
            let net_uri = self.connection_map.get(&dht_uri).expect("unknown dht_uri");
            conn_list.push(net_uri);
            trace!(
                "({}).send() reversed mapped dht_uri {:?} to net_uri {:?}",
                self.identifier,
                dht_uri,
                net_uri,
            )
        }
        let ref_list: Vec<&str> = conn_list.iter().map(|v| v.as_str()).collect();
        // Send on the inner Transport
        self.inner_transport.as_mut().send(&ref_list, payload)
        */
        Ok(())
    }

    /// Get Uris from DHT peer_address'
    pub(crate) fn dht_address_to_uri_list(
        &self,
        address_list: &[&str],
    ) -> TransportResult<Vec<Url>> {
        let mut uri_list = Vec::with_capacity(address_list.len());
        for address in address_list {
            let maybe_peer = self.inner_dht.get_peer(address);
            match maybe_peer {
                None => {
                    return Err(TransportError::new(format!(
                        "Unknown peerAddress: {}",
                        address
                    )));
                }
                Some(peer) => uri_list.push(peer.peer_uri),
            }
        }
        Ok(uri_list)
    }

    fn handle_new_connection(&mut self, address: Url) -> TransportResult<()> {
        // TODO #176 - Maybe we shouldn't have different code paths for populating
        // the connection_map between space and network gateways.
        self.connection.insert(address.clone);

        // Send to other node our PeerAddress
        let this_peer = self.this_peer().clone();
        let our_peer_address = P2pProtocol::PeerAddress(
            self.identifier().to_string(),
            this_peer.peer_address,
            this_peer.timestamp,
        );
        let mut buf = Vec::new();
        our_peer_address
            .serialize(&mut Serializer::new(&mut buf))
            .unwrap();
        trace!(
            "({}) sending P2pProtocol::PeerAddress: {:?} to {:?}",
            self.identifier,
            our_peer_address,
            address,
        );
        return self.inner_transport.as_mut().send(
            "".to_string(), address, buf);
    }

    /// Process a transportEvent received from our internal connection.
    pub(crate) fn handle_TransportEvent(&mut self, evt: &TransportEvent) -> TransportResult<()> {
        debug!(
            "<<< '({})' recv transport event: {:?}",
            self.identifier, evt
        );
        // Note: use same order as the enum
        /*
        match evt {
            TransportEvent::ErrorOccured(id, e) => {
                error!(
                    "({}) Connection Error for {}: {}\n Closing connection.",
                    self.identifier, id, e,
                );
                self.inner_transport.as_mut().close(id)?;
            }
            TransportEvent::ConnectResult(id, _) => {
                info!("({}) Outgoing connection opened: {}", self.identifier, id);
                self.handle_new_connection(id)?;
            }
            TransportEvent::IncomingConnectionEstablished(id) => {
                info!("({}) Incoming connection opened: {}", self.identifier, id);
                self.handle_new_connection(id)?;
            }
            TransportEvent::ConnectionClosed(_id) => {
                // TODO #176
            }
            TransportEvent::ReceivedData(connection_id, payload) => {
                debug!("Received message from: {}", connection_id);
                // trace!("Deserialize msg: {:?}", payload);
                let mut de = Deserializer::new(&payload[..]);
                let maybe_p2p_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                if let Ok(p2p_msg) = maybe_p2p_msg {
                    if let P2pProtocol::PeerAddress(gateway_id, peer_address, peer_timestamp) =
                        p2p_msg
                    {
                        debug!(
                            "Received PeerAddress: {} | {} ({})",
                            peer_address, gateway_id, self.identifier
                        );
                        let peer_uri = self
                            .inner_transport
                            .as_mut()
                            .get_uri(connection_id)
                            .expect("FIXME"); // TODO #58
                        debug!("peer_uri of: {} = {}", connection_id, peer_uri);
                        if self.identifier == gateway_id {
                            let peer = PeerData {
                                peer_address: peer_address.clone(),
                                peer_uri,
                                timestamp: peer_timestamp,
                            };
                            Dht::post(self, DhtCommand::HoldPeer(peer)).expect("FIXME"); // TODO #58
                                                                                         // TODO #150 - Should not call process manually
                            Dht::process(self).expect("HACK");
                        }
                    }
                }
            }
        };
        */
        Ok(())
    }

    /// Process a TransportCommand: Call the corresponding method and possibily return some Events.
    /// Return a list of TransportEvents to owner.
    #[allow(non_snake_case)]
    fn serve_TransportCommand(
        &mut self,
        cmd: &TransportCommand,
    ) -> TransportResult<()> {
        trace!("({}) serving transport cmd: {:?}", self.identifier, cmd);
        // Note: use same order as the enum
        match cmd {
            TransportCommand::Bind { request_id, spec } => {
                self.priv_bind(request_id, spec)
            }
            TransportCommand::Connect { request_id, address } => {
                self.priv_connect(request_id, address)
            }
            TransportCommand::SendMessage { request_id, address, payload } => {
                self.priv_send(request_id, address, payload)
            }
        }
    }
}
