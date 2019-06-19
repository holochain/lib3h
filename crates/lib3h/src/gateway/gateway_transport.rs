#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    engine::p2p_protocol::P2pProtocol,
    gateway::P2pGateway,
    transport::{
        error::{TransportError, TransportResult},
        protocol::{TransportCommand, TransportEvent},
        transport_trait::Transport,
        TransportId, TransportIdRef,
    },
};
use lib3h_protocol::DidWork;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

/// Compose Transport
impl<T: Transport, D: Dht> Transport for P2pGateway<T, D> {
    fn connect(&mut self, uri: &str) -> TransportResult<TransportId> {
        self.inner_transport.borrow_mut().connect(&uri)
    }
    fn close(&mut self, id: &TransportIdRef) -> TransportResult<()> {
        self.inner_transport.borrow_mut().close(id)
    }

    fn close_all(&mut self) -> TransportResult<()> {
        self.inner_transport.borrow_mut().close_all()
    }

    //    fn send(&mut self, id_list: &[&TransportIdRef], payload: &[u8]) -> TransportResult<()> {
    //        // get peer transport from dht first
    //        let mut transport_list = self.address_to_transport_list(id_list);
    //        // send
    //        println!("[t] (Gateway).send() {:?} | {}", id_list, payload.len());
    //        let ref_list: Vec<&str> = transport_list.iter().map(|v| &**v).collect();
    //        self.inner_transport.borrow_mut().send(&ref_list, payload)
    //    }

    fn send(&mut self, id_list: &[&TransportIdRef], payload: &[u8]) -> TransportResult<()> {
        self.inner_transport.borrow_mut().send(&id_list, payload)
    }

    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        self.inner_transport.borrow_mut().send_all(payload)
    }

    fn bind(&mut self, url: &str) -> TransportResult<String> {
        self.inner_transport.borrow_mut().bind(url)
    }

    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.inner_transport.borrow_mut().post(command)
    }

    /// FIXME: Should P2pGateway handle TransportEvents directly?
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        //self.inner_transport.process()
        let (did_work, event_list) = self.inner_transport.borrow_mut().process()?;
        if did_work {
            for evt in event_list.clone() {
                self.handle_TransportEvent(&evt)?;
            }
        }
        Ok((did_work, event_list))
    }

    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        self.inner_transport.borrow().transport_id_list()
    }

    fn get_uri(&self, id: &TransportIdRef) -> Option<String> {
        self.inner_transport.borrow().get_uri(id)
    }
}

/// Private internals
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    /// Process a transportEvent received from our internal connection.
    pub(crate) fn handle_TransportEvent(&mut self, evt: &TransportEvent) -> TransportResult<()> {
        println!("[d] <<< '(GatewayTransport)' recv: {:?}", evt);
        // Note: use same order as the enum
        match evt {
            TransportEvent::TransportError(id, e) => {
                println!(
                    "[e] Connection Error for {}: {}\n Closing connection.",
                    id, e
                );
                self.inner_transport.borrow_mut().close(id)?;
            }
            TransportEvent::ConnectResult(id) => {
                println!("[i] Connection opened id: {}", id);
                if let Some(uri) = self.get_uri(id) {
                    println!("[i] Connection opened uri: {}", uri);

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
                                peer_address,
                                transport: id.clone(),
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
}
