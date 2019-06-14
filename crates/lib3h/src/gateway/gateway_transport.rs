#![allow(non_snake_case)]

use crate::{
    dht::dht_trait::Dht,
    engine::p2p_protocol::P2pProtocol,
    gateway::p2p_gateway::P2pGateway,
    transport::{
        error::{TransportError, TransportResult},
        memory_mock::transport_memory::TransportMemory,
        protocol::{TransportCommand, TransportEvent},
        transport_trait::Transport,
        TransportId, TransportIdRef,
    },
};
use lib3h_protocol::{data_types::EntryData, AddressRef, DidWork, Lib3hResult};

/// Compose Transport
impl<'t, T: Transport, D: Dht> Transport for P2pGateway<'t, T, D> {
    fn connect(&mut self, uri: &str) -> TransportResult<TransportId> {
        self.inner_transport.connect(&uri)
    }
    fn close(&mut self, id: &TransportIdRef) -> TransportResult<()> {
        self.inner_transport.close(id)
    }

    fn close_all(&mut self) -> TransportResult<()> {
        self.inner_transport.close_all()
    }

    fn send(&mut self, id_list: &[&TransportIdRef], payload: &[u8]) -> TransportResult<()> {
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
                Some(peer) => transport_list.push(peer.transport.as_str()),
            }
        }
        // send
        self.inner_transport.send(&transport_list, payload)
    }

    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        self.inner_transport.send_all(payload)
    }

    fn bind(&mut self, url: &str) -> TransportResult<String> {
        let res = self.inner_transport.bind(url);
        if let Ok(adv) = res.clone() {
            self.maybe_advertise = Some(adv);
        }
        res
    }

    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.inner_transport.post(command)
    }

    /// FIXME: Should P2pGateway handle TransportEvents directly?
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        //self.inner_transport.process()
        let (did_work, event_list) = self.inner_transport.process()?;
        if did_work {
            for evt in event_list.clone() {
                self.handle_TransportEvent(&evt)?;
            }
        }
        Ok((did_work, event_list))
    }

    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        self.inner_transport.transport_id_list()
    }
}

/// Private internals
impl<'t, T: Transport, D: Dht> P2pGateway<'t, T, D> {
    /// Process a transportEvent received from our internal connection.
    pub(crate) fn handle_TransportEvent(&mut self, evt: &TransportEvent) -> TransportResult<()> {
        println!("(log.d) >>> '(TransportGateway)' recv: {:?}", evt);
        // Note: use same order as the enum
        match evt {
            TransportEvent::TransportError(id, e) => {
                println!(
                    "(log.e) Connection Error for {}: {}\n Closing connection.",
                    id, e
                );
                self.inner_transport.close(id)?;
            }
            TransportEvent::ConnectResult(id) => {
                // don't need to do anything here
                println!("(log.i) Connection opened: {}", id);
            }
            TransportEvent::Closed(id) => {
                // FIXME
                println!("(log.w) Connection closed: {}", id);
                self.inner_transport.close(id)?;
                //let _transport_id = self.wss_socket.wait_connect(&self.ipc_uri)?;
            }
            TransportEvent::Received(id, msg) => {
                println!("(log.d) Received message from: {}", id);
            }
        };
        Ok(())
    }
}
