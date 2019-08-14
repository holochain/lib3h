#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::p2p_protocol::P2pProtocol,
    gateway::{Gateway, P2pGateway},
    transport::{
        error::{TransportError, TransportResult},
        protocol::{TransportCommand, TransportEvent},
        transport_trait::Transport,
        ConnectionId, ConnectionIdRef,
    },
};
use lib3h_protocol::{data_types::*, DidWork};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;

/// Compose Transport
impl<'gateway, D: Dht> Transport for P2pGateway<'gateway, D> {
    // TODO #176 - Return a higher-level uri instead?
    fn connect(&mut self, uri: &Url) -> TransportResult<ConnectionId> {
        trace!("({}).connect() {}", self.identifier, uri);
        // Connect
        let connection_id = self.inner_transport.as_mut().connect(&uri)?;
        // Store result in connection map
        self.connection_map
            .insert(uri.clone(), connection_id.clone());
        // Done
        Ok(connection_id)
    }

    // TODO #176 - remove conn id conn_map??
    fn close(&mut self, id: &ConnectionIdRef) -> TransportResult<()> {
        self.inner_transport.as_mut().close(id)
    }

    // TODO #176
    fn close_all(&mut self) -> TransportResult<()> {
        self.inner_transport.as_mut().close_all()
    }

    /// id_list =
    ///   - Network : transportId
    ///   - space   : agentId
    fn send(&mut self, dht_id_list: &[&ConnectionIdRef], payload: &[u8]) -> TransportResult<()> {
        for id in dht_id_list {
            let uri = self.inner_dht.get_peer(id);
            if uri.is_none() {
                return Err(TransportError::new(format!("Unknown peerAddress: {}", id)));
            }
            let uri = uri.unwrap().peer_uri;

            let con_id = self.connection_map.get(&uri).expect("unknown dht uri");

            let mut inner_payload = Vec::new();
            P2pProtocol::DirectMessage(DirectMessageData {
                space_address: self.space_address.clone(),
                request_id: "".to_string(),
                to_agent_id: id.to_string().into(),
                from_agent_id: self.inner_dht.this_peer().peer_address.clone().into(),
                content: payload.to_vec(),
            })
            .serialize(&mut rmp_serde::Serializer::new(&mut inner_payload))
            .expect("serialization failed");

            self.inner_transport
                .as_mut()
                .send(&[&con_id], &inner_payload)?;
        }

        Ok(())

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
    }

    ///
    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        let connection_list = self.connection_id_list()?;
        let dht_id_list: Vec<&str> = connection_list.iter().map(|v| &**v).collect();
        trace!("({}) send_all() {:?}", self.identifier, dht_id_list);
        self.send(&dht_id_list, payload)
    }

    ///
    fn bind(&mut self, url: &Url) -> TransportResult<Url> {
        trace!("({}) bind() {}", self.identifier, url);
        self.inner_transport.as_mut().bind(url)
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
        for evt in outbox.drain(..).collect::<Vec<_>>() {
            if self.handle_TransportEvent(&evt)? {
                outbox.push(evt);
            }
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
        self.inner_transport.as_ref().get_uri(id)
        //let maybe_peer_data = self.inner_dht.get_peer(id);
        //maybe_peer_data.map(|pd| pd.peer_address)
    }
}

/// Private internals
impl<'gateway, D: Dht> P2pGateway<'gateway, D> {
    /*
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
    */

    fn handle_new_connection(&mut self, id: &ConnectionIdRef) -> TransportResult<()> {
        let maybe_uri = self.get_uri(id);
        if maybe_uri.is_none() {
            return Ok(());
        }
        let uri = maybe_uri.unwrap();
        trace!("({}) new_connection: {} -> {}", self.identifier, uri, id,);
        // TODO #176 - Maybe we shouldn't have different code paths for populating
        // the connection_map between space and network gateways.
        let maybe_previous = self.connection_map.insert(uri.clone(), id.to_string());
        if let Some(previous_cId) = maybe_previous {
            debug!("Replaced connectionId for {} ; was: {}", uri, previous_cId,);
        }

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
            id,
        );
        return self.inner_transport.as_mut().send(&[&id], &buf);
    }

    /// Process a transportEvent received from our internal connection.
    pub(crate) fn handle_TransportEvent(&mut self, evt: &TransportEvent) -> TransportResult<bool> {
        debug!(
            "<<< '({})' recv transport event: {:?}",
            self.identifier, evt
        );
        // Note: use same order as the enum
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
                    return self.handle_TransportEvent_P2pProtocol(connection_id, &p2p_msg);
                }
            }
        };
        Ok(true)
    }

    /// Process a transportEvent received from our internal connection.
    fn handle_TransportEvent_P2pProtocol(
        &mut self,
        connection_id: &ConnectionIdRef,
        p2p_msg: &P2pProtocol,
    ) -> TransportResult<bool> {
        match p2p_msg {
            P2pProtocol::PeerAddress(gateway_id, peer_address, peer_timestamp) => {
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
                if &self.identifier == gateway_id {
                    let peer = PeerData {
                        peer_address: peer_address.clone(),
                        peer_uri,
                        timestamp: *peer_timestamp,
                    };
                    Dht::post(self, DhtCommand::HoldPeer(peer)).expect("FIXME"); // TODO #58
                                                                                 // TODO #150 - Should not call process manually
                    Dht::process(self).expect("HACK");
                }
                Ok(false)
            }
            P2pProtocol::DirectMessage(dm_data) => {
                if dm_data.space_address != self.space_address {
                    return Err(format!(
                        "Gateway DM: Bad Space Address, wanted {} got {}",
                        self.space_address, dm_data.space_address
                    )
                    .into());
                }
                error!("{:?}", dm_data);
                self.transport_outbox
                    .entry(dm_data.to_agent_id.clone())
                    .or_insert_with(|| Vec::new())
                    .push(TransportEvent::ReceivedData(
                        "".to_string(),
                        dm_data.content.clone(),
                    ));
                std::process::exit(127);
                Ok(false)
            }
            _ => Ok(true),
        }
    }

    /// Process a TransportCommand: Call the corresponding method and possibily return some Events.
    /// Return a list of TransportEvents to owner.
    #[allow(non_snake_case)]
    fn serve_TransportCommand(
        &mut self,
        cmd: &TransportCommand,
    ) -> TransportResult<Vec<TransportEvent>> {
        trace!("({}) serving transport cmd: {:?}", self.identifier, cmd);
        // Note: use same order as the enum
        match cmd {
            TransportCommand::Connect(url, request_id) => {
                let id = self.connect(url)?;
                let evt = TransportEvent::ConnectResult(id, request_id.clone());
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
                let evt = TransportEvent::ConnectionClosed(id.to_string());
                Ok(vec![evt])
            }
            TransportCommand::CloseAll => {
                self.close_all()?;
                let outbox = Vec::new();
                Ok(outbox)
            }
            TransportCommand::Bind(url) => {
                self.bind(url)?;
                Ok(vec![])
            }
        }
    }
}
