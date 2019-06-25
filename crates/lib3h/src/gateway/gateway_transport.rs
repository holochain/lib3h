#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::p2p_protocol::P2pProtocol,
    gateway::P2pGateway,
    transport::{
        error::{TransportError, TransportResult},
        protocol::{TransportCommand, TransportEvent},
        transport_id_to_url,
        transport_trait::Transport,
        TransportId, TransportIdRef,
    },
};
use lib3h_protocol::DidWork;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;

/// Compose Transport
impl<T: Transport, D: Dht> Transport for P2pGateway<T, D> {
    /// TODO: return a higher-level uri instead
    fn connect(&mut self, uri: &Url) -> TransportResult<TransportId> {
        println!("[t] ({}).connect() {}", self.identifier.clone(), uri);
        // Connect
        let transport_id = self.inner_transport.borrow_mut().connect(&uri)?;
        // Store result in reverse map
        println!("[t] ({}).connect() reverse mapping uri {} to {}", self.identifier.clone(),
            uri, transport_id);
        self.reverse_map.insert(uri.clone(), transport_id.clone());
        // Done
        Ok(transport_id)
    }

    /// TODO?
    fn close(&mut self, id: &TransportIdRef) -> TransportResult<()> {
        // FIXME remove conn id conn_map??
        self.inner_transport.borrow_mut().close(id)
    }

    /// TODO?
    fn close_all(&mut self) -> TransportResult<()> {
        self.inner_transport.borrow_mut().close_all()
    }

    /// id_list =
    ///   - Network : machine_id
    ///   - space   : agent_id
    fn send(&mut self, dht_id_list: &[&TransportIdRef], payload: &[u8]) -> TransportResult<()> {
        // get transportId from the inner dht first
        let dht_transport_list = self.address_to_dht_transport_list(dht_id_list)?;
        // send
        println!(
            "[t] ({}).send() {:?} -> {:?} | {}",
            self.identifier.clone(),
            dht_id_list,
            dht_transport_list,
            payload.len()
        );
        // Get transportIds for the inner Transport.
        let mut net_transport_list = Vec::new();
        for dht_transport in dht_transport_list {
            let net_transport = self
                .reverse_map
                .get(&dht_transport)
                .expect("unknown dht_transport");
            net_transport_list.push(net_transport);
            println!("[t] ({}).send() reversed mapped dht_transport {:?} to net_transport {:?}",
                self.identifier.clone(), dht_transport, net_transport)
        }
        let ref_list: Vec<&str> = net_transport_list.iter().map(|v| v.as_str()).collect();
        // Send on the inner Transport
        self.inner_transport.borrow_mut().send(&ref_list, payload)
    }

    ///
    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        let transport_list = self.transport_id_list()?;
        let transport_list: Vec<&str> = transport_list.iter().map(|v| &**v).collect();
        println!(
            "[t] ({}) send_all() {:?}",
            self.identifier.clone(),
            transport_list
        );
        self.send(&transport_list, payload)
    }

    /// TODO?
    fn bind(&mut self, url: &Url) -> TransportResult<Url> {
        println!("[t] ({}) bind() {}", self.identifier.clone(), url);
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
        println!(
            "[t] ({}).Transport.process() - output: {} {}",
            self.identifier.clone(),
            did_work,
            outbox.len()
        );
        //// ?? Don't process inner transport because we are not the owner and are not responsable for that??
        // Process inner transport
        let (inner_did_work, mut event_list) = self.inner_transport.borrow_mut().process()?;
        println!(
            "[t] ({}).Transport.inner_process() - output: {} {}",
            self.identifier.clone(),
            inner_did_work,
            event_list.len()
        );
        if inner_did_work {
            did_work = true;
            outbox.append(&mut event_list);
        }
        // Handle TransportEvents
        for evt in outbox.clone() {
            self.handle_TransportEvent(&evt)?;
        }
        Ok((did_work, outbox))
    }

    /// A Gateway uses as transportId its inner_dht's peerData.peer_address
    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        let peer_data_list = self.inner_dht.get_peer_list();
        let mut id_list = Vec::new();
        for peer_data in peer_data_list {
            id_list.push(peer_data.peer_address);
        }
        Ok(id_list)
    }

    /// TODO: return a higher-level uri instead
    fn get_uri(&self, id: &TransportIdRef) -> Option<Url> {
        self.inner_transport.borrow().get_uri(id)
        //let maybe_peer_data = self.inner_dht.get_peer(id);
        //maybe_peer_data.map(|pd| pd.peer_address)
    }
}

/// Private internals
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    /// Get Transports from DHT peer_address'
    pub(crate) fn address_to_dht_transport_list(
        &self,
        id_list: &[&TransportIdRef],
    ) -> TransportResult<Vec<Url>> {
        // get peer transport from dht first
        let mut transport_list = Vec::with_capacity(id_list.len());
        for transportId in id_list {
            let maybe_peer = self.inner_dht.get_peer(transportId);
            match maybe_peer {
                None => {
                    return Err(TransportError::new(format!(
                        "Unknown transportId: {}",
                        transportId
                    )));
                }
                Some(peer) => transport_list.push(peer.transport),
            }
        }
        Ok(transport_list)
    }

    /// Process a transportEvent received from our internal connection.
    pub(crate) fn handle_TransportEvent(&mut self, evt: &TransportEvent) -> TransportResult<()> {
        println!(
            "[d] <<< '({})' recv transport event: {:?}",
            self.identifier.clone(),
            evt
        );
        // Note: use same order as the enum
        match evt {
            TransportEvent::TransportError(id, e) => {
                println!(
                    "[e] (GatewayTransport) Connection Error for {}: {}\n Closing connection.",
                    id, e
                );
                self.inner_transport.borrow_mut().close(id)?;
            }
            TransportEvent::ConnectResult(id) => {
                println!(
                    "[i] ({}) Connection opened id: {}",
                    self.identifier.clone(),
                    id
                );
                if let Some(uri) = self.get_uri(id) {
                    println!("[i] (GatewayTransport) Connection opened uri: {}", uri);

                    println!(
                        "[t] (GatewayTransport).ConnectResult: mapping {} -> {}",
                        uri, id
                    );
                    self.reverse_map.insert(uri, id.clone());
                    // Ok(conn_id)

                    // Send it our PeerAddress
                    let our_peer_address = P2pProtocol::PeerAddress(
                        self.identifier().to_string(),
                        self.this_peer().clone().peer_address,
                    );
                    let mut buf = Vec::new();
                    our_peer_address
                        .serialize(&mut Serializer::new(&mut buf))
                        .unwrap();
                    println!(
                        "(GatewayTransport) P2pProtocol::PeerAddress: {:?} to {:?}",
                        our_peer_address, id
                    );
                    // Send our PeerAddress to other side
                    self.inner_transport.borrow_mut().send(&[&id], &buf)?;
                }
            }
            TransportEvent::Connection(_id) => {
                // TODO!!
                unimplemented!();
            }
            TransportEvent::Closed(id) => {
                // FIXME
                println!("[w] Connection closed: {}", id);
                self.inner_transport.borrow_mut().close(id)?;
                //let _transport_id = self.wss_socket.wait_connect(&self.ipc_uri)?;
            }
            TransportEvent::Received(id, payload) => {
                println!("[d] Received message from: {}", id);
                // println!("Deserialize msg: {:?}", payload);
                let mut de = Deserializer::new(&payload[..]);
                let maybe_p2p_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                if let Ok(p2p_msg) = maybe_p2p_msg {
                    if let P2pProtocol::PeerAddress(gateway_id, peer_address) = p2p_msg {
                        println!(
                            "[d] Received PeerAddress: {} | {} ({})",
                            peer_address, gateway_id, self.identifier
                        );
                        if self.identifier == gateway_id {
                            let peer = PeerData {
                                peer_address: peer_address.clone(),
                                transport: transport_id_to_url(id.clone()),
                                timestamp: 42, // FIXME
                            };
                            Dht::post(self, DhtCommand::HoldPeer(peer)).expect("FIXME");
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
        println!(
            "[d] >>> '({})' recv transport cmd: {:?}",
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
