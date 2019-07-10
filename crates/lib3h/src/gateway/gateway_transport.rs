#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::p2p_protocol::P2pProtocol,
    gateway::P2pGateway,
    transport::{
        error::{TransportError, TransportResult},
        protocol::{TransportCommand, TransportEvent},
        transport_trait::Transport,
        ConnectionId, ConnectionIdRef,
    },
};
use lib3h_protocol::DidWork;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;

// TODO make a struct for transport id and make these trait converters
fn transport_id_to_url(id: ConnectionId) -> Url {
    Url::parse(id.as_str())
        .expect("gateway_transport: transport_id_to_url: connection id is not a well formed url")
}

/// Compose Transport
impl<T: Transport, D: Dht> Transport for P2pGateway<T, D> {
    /// TODO: return a higher-level uri instead
    fn connect(&mut self, uri: &Url) -> TransportResult<ConnectionId> {
        trace!("({}).connect() {}", self.identifier.clone(), uri);
        // Connect
        let connection_id = self.inner_transport.borrow_mut().connect(&uri)?;
        // Store result in connection map
        self.connection_map
            .insert(uri.clone(), connection_id.clone());
        // Done
        Ok(connection_id)
    }

    /// TODO?
    fn close(&mut self, id: &ConnectionIdRef) -> TransportResult<()> {
        // FIXME remove conn id conn_map??
        self.inner_transport.borrow_mut().close(id)
    }

    /// TODO?
    fn close_all(&mut self) -> TransportResult<()> {
        self.inner_transport.borrow_mut().close_all()
    }

    /// id_list =
    ///   - Network : transportId
    ///   - space   : agentId
    fn send(&mut self, dht_id_list: &[&ConnectionIdRef], payload: &[u8]) -> TransportResult<()> {
        // get connectionId from the inner dht first
        let dht_uri_list = self.dht_address_to_uri_list(dht_id_list)?;
        // send
        trace!(
            "({}).send() {:?} -> {:?} | {}",
            self.identifier.clone(),
            dht_id_list,
            dht_uri_list,
            payload.len()
        );
        // Get connectionIds for the inner Transport.
        let mut conn_list = Vec::new();
        for dht_uri in dht_uri_list {
            let net_uri = self.connection_map.get(&dht_uri).expect("unknown dht_uri");
            conn_list.push(net_uri);
            trace!(
                "({}).send() reversed mapped dht_uri {:?} to net_uri {:?}",
                self.identifier.clone(),
                dht_uri,
                net_uri
            )
        }
        let ref_list: Vec<&str> = conn_list.iter().map(|v| v.as_str()).collect();
        // Send on the inner Transport
        self.inner_transport.borrow_mut().send(&ref_list, payload)
    }

    ///
    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        let connection_list = self.connection_id_list()?;
        let dht_id_list: Vec<&str> = connection_list.iter().map(|v| &**v).collect();
        trace!("({}) send_all() {:?}", self.identifier.clone(), dht_id_list);
        self.send(&dht_id_list, payload)
    }

    /// TODO?
    fn bind(&mut self, url: &Url) -> TransportResult<Url> {
        trace!("({}) bind() {}", self.identifier.clone(), url);
        self.inner_transport.borrow_mut().bind(url)
    }

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
            self.identifier.clone(),
            did_work,
            outbox.len()
        );
        //// ?? Don't process inner transport because we are not the owner and are not responsable for that??
        // Process inner transport
        let (inner_did_work, mut event_list) = self.inner_transport.borrow_mut().process()?;
        trace!(
            "({}).Transport.inner_process() - output: {} {}",
            self.identifier.clone(),
            inner_did_work,
            event_list.len()
        );
        if inner_did_work {
            did_work = true;
            outbox.append(&mut event_list);
        }
        // Handle TransportEvents
        for evt in outbox.clone().iter().rev() {
            self.handle_TransportEvent(&evt)?;
        }
        Ok((did_work, outbox))
    }

    /// A Gateway uses its inner_dht's peerData.peer_address as connectionId
    fn connection_id_list(&self) -> TransportResult<Vec<ConnectionId>> {
        let peer_data_list = self.inner_dht.get_peer_list();
        let mut id_list = Vec::new();
        for peer_data in peer_data_list {
            id_list.push(peer_data.peer_address);
        }
        Ok(id_list)
    }

    /// TODO: return a higher-level uri instead
    fn get_uri(&self, id: &ConnectionIdRef) -> Option<Url> {
        self.inner_transport.borrow().get_uri(id)
        //let maybe_peer_data = self.inner_dht.get_peer(id);
        //maybe_peer_data.map(|pd| pd.peer_address)
    }
}

/// Private internals
impl<T: Transport, D: Dht> P2pGateway<T, D> {
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

    /// Process a transportEvent received from our internal connection.
    pub(crate) fn handle_TransportEvent(&mut self, evt: &TransportEvent) -> TransportResult<()> {
        debug!(
            "<<< '({})' recv transport event: {:?}",
            self.identifier.clone(),
            evt
        );
        // Note: use same order as the enum
        match evt {
            TransportEvent::TransportError(id, e) => {
                error!(
                    "(GatewayTransport) Connection Error for {}: {}\n Closing connection.",
                    id, e
                );
                self.inner_transport.borrow_mut().close(id)?;
            }
            TransportEvent::ConnectResult(id) => {
                info!("({}) Connection opened id: {}", self.identifier.clone(), id);
                let maybe_uri = self.get_uri(id);
                if maybe_uri.is_none() {
                    return Ok(());
                }
                let uri = maybe_uri.unwrap();
                trace!(
                    "(GatewayTransport).ConnectResult: mapping {} -> {}",
                    uri,
                    id
                );
                self.connection_map.insert(uri, id.clone());

                // Send to other node our PeerAddress
                let our_peer_address = P2pProtocol::PeerAddress(
                    self.identifier().to_string(),
                    self.this_peer().clone().peer_address,
                );
                let mut buf = Vec::new();
                our_peer_address
                    .serialize(&mut Serializer::new(&mut buf))
                    .unwrap();
                trace!(
                    "(GatewayTransport) P2pProtocol::PeerAddress: {:?} to {:?}",
                    our_peer_address,
                    id
                );
                self.inner_transport.borrow_mut().send(&[&id], &buf)?;
            }
            TransportEvent::Connection(_id) => {
                // TODO!!
                unimplemented!();
            }
            TransportEvent::Closed(id) => {
                // FIXME
                warn!("Connection closed: {}", id);
                self.inner_transport.borrow_mut().close(id)?;
                //let _transport_id = self.wss_socket.wait_connect(&self.ipc_uri)?;
            }
            TransportEvent::Received(id, payload) => {
                debug!("Received message from: {}", id);
                // debug!("Deserialize msg: {:?}", payload);
                let mut de = Deserializer::new(&payload[..]);
                let maybe_p2p_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                if let Ok(p2p_msg) = maybe_p2p_msg {
                    if let P2pProtocol::PeerAddress(gateway_id, peer_address) = p2p_msg {
                        debug!(
                            "Received PeerAddress: {} | {} ({})",
                            peer_address, gateway_id, self.identifier
                        );
                        if self.identifier == gateway_id {
                            let peer = PeerData {
                                peer_address: peer_address.clone(),
                                peer_uri: transport_id_to_url(id.clone()),
                                timestamp: 42, // FIXME
                            };
                            Dht::post(self, DhtCommand::HoldPeer(peer)).expect("FIXME");
                            // FIXME: Should not call process manually
                            Dht::process(self).expect("HACK");
                        }
                    }
                }
            }
        };
        Ok(())
    }

    /// Process a TransportCommand: Call the corresponding method and possibily return some Events.
    /// Return a list of TransportEvents to owner.
    #[allow(non_snake_case)]
    fn serve_TransportCommand(
        &mut self,
        cmd: &TransportCommand,
    ) -> TransportResult<Vec<TransportEvent>> {
        trace!(
            "({}) serving transport cmd: {:?}",
            self.identifier.clone(),
            cmd
        );
        // Note: use same order as the enum
        match cmd {
            TransportCommand::Connect(url) => {
                let id = self.connect(url)?;
                let evt = TransportEvent::ConnectResult(id);
                Ok(vec![evt])
            }
            TransportCommand::Send(id_list, payload) => {
                let mut id_ref_list = Vec::with_capacity(id_list.len());
                for id in id_list {
                    id_ref_list.push(id.as_str());
                }
                let _id = self.send(&id_ref_list, payload)?;
                Ok(vec![])
            }
            TransportCommand::SendAll(payload) => {
                let _id = self.send_all(payload)?;
                Ok(vec![])
            }
            TransportCommand::Close(id) => {
                self.close(id)?;
                let evt = TransportEvent::Closed(id.to_string());
                Ok(vec![evt])
            }
            TransportCommand::CloseAll => {
                self.close_all()?;
                let outbox = Vec::new();
                // FIXME: Send Closed event for each connection
                //                for (id, _url) in &self.connections {
                //                    let evt = TransportEvent::Closed(id.to_string());
                //                    outbox.push(evt);
                //                }
                Ok(outbox)
            }
            TransportCommand::Bind(url) => {
                self.bind(url)?;
                Ok(vec![])
            }
        }
    }
}
