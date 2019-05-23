#![allow(non_snake_case)]

use crate::{
    dht::{
        dht_event::{DhtEvent, PeerHoldRequestData},
        dht_trait::Dht,
        rrdht::RrDht,
    },
    p2p::p2p_protocol::P2pProtocol,
    transport::{Transport, TransportEvent, TransportId, TransportIdRef, TransportResult},
    transport_wss::TransportWss,
};
use holochain_lib3h_protocol::{Address, DidWork, Lib3hResult};

/// Gateway to a P2P network.
/// Enables Connections to many other nodes.
/// Tracks distributed data for that P2P network.
pub struct P2pGateway {
    transport_connection: TransportWss<std::net::TcpStream>,
    dht: Box<Dht>,
}

/// Public interface
impl P2pGateway {
    /// Constructor
    pub fn new() -> Self {
        P2pGateway {
            transport_connection: TransportWss::with_std_tcp_stream(),
            dht: Box::new(RrDht::new()),
        }
    }

    // -- Getters -- //

    /// This nodes identifier on the network
    pub fn id(&self) -> String {
        // FIXME
        "FIXME".to_string()
    }

    /// This nodes connection address
    pub fn advertise(&self) -> String {
        // FIXME
        "FIXME".to_string()
    }
}

/// Compose DHT
impl Dht for P2pGateway {
    /// Peer info
    fn get_peer(&self, peer_address: String) -> Option<PeerHoldRequestData> {
        self.dht.get_peer(peer_address)
    }
    fn fetch_peer(&self, peer_address: String) -> Option<PeerHoldRequestData> {
        self.dht.fetch_peer(peer_address)
    }
    fn drop_peer(&self, peer_address: String) -> Lib3hResult<()> {
        self.dht.drop_peer(peer_address)
    }
    /// Data
    fn get_data(&self, data_address: Address) -> Lib3hResult<Vec<u8>> {
        self.dht.get_data(data_address)
    }
    fn fetch_data(&self, data_address: Address) -> Lib3hResult<Vec<u8>> {
        self.dht.fetch_data(data_address)
    }
    /// Processing
    fn post(&mut self, evt: DhtEvent) -> Lib3hResult<()> {
        self.dht.post(evt)
    }
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)> {
        self.dht.process()
    }
    /// Getters
    fn this_peer(&self) -> Lib3hResult<()> {
        self.dht.this_peer()
    }
}

/// Compose Transport
impl Transport for P2pGateway {
    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        self.transport_connection.transport_id_list()
    }

    fn connect(&mut self, uri: &str) -> TransportResult<TransportId> {
        self.transport_connection.wait_connect(&uri)
    }
    fn close(&mut self, id: TransportId) -> TransportResult<()> {
        self.transport_connection.close(id)
    }

    fn close_all(&mut self) -> TransportResult<()> {
        self.transport_connection.close_all()
    }

    fn send(&mut self, id_list: &[&TransportIdRef], payload: &[u8]) -> TransportResult<()> {
        self.transport_connection.send(id_list, payload)
    }

    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        self.transport_connection.send_all(payload)
    }

    fn poll(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        Ok(self.transport_connection.poll()?)
    }
}

/// Public - specific
impl P2pGateway {
    pub fn process(&mut self) -> Lib3hResult<(DidWork, Vec<P2pProtocol>)> {
        let mut outbox = Vec::new();
        // Process the transport connection
        let (did_work, transport_event_list) = self.poll()?;
        if did_work {
            for evt in transport_event_list {
                let (did_work, mut p2p_output) = self.serve_TransportEvent(&evt)?;
                // Add p2p events to outbox
                if did_work {
                    outbox.append(&mut p2p_output);
                }
            }
        }
        // Process the transport dht
        let (did_work, dht_event_list) = self.dht.process()?;
        if did_work {
            for evt in dht_event_list {
                let (did_work, mut p2p_output) = self.serve_DhtEvent(evt)?;
                // Add p2p events to outbox
                if did_work {
                    outbox.append(&mut p2p_output);
                }
            }
        }
        // Done
        Ok((did_work, outbox))
    }
}

/// Private internals
impl P2pGateway {
    /// Process a transportEvent.
    /// Return a list of P2pProtocol messages to send to others.
    fn serve_TransportEvent(
        &mut self,
        evt: &TransportEvent,
    ) -> Lib3hResult<(DidWork, Vec<P2pProtocol>)> {
        println!("(log.d) >>> '(TransportGateway)' recv: {:?}", evt);
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Note: use same order as the enum
        match evt {
            TransportEvent::TransportError(id, _e) => {
                println!("(log.w) Connection Error: {}\n Closing connection.", id);
                self.transport_connection.close(id.to_string())?;
            }
            TransportEvent::Connect(id) => {
                // don't need to do anything here
                println!("(log.i) Connection opened: {}", id);
            }
            TransportEvent::Close(id) => {
                // FIXME
                println!("(log.w) Connection closed: {}", id);
                self.transport_connection.close(id.to_string())?;
                //let _transport_id = self.wss_socket.wait_connect(&self.ipc_uri)?;
            }
            TransportEvent::Message(_id, _msg) => {
                // FIXME: Make sense of msg?
            }
        };
        Ok((did_work, outbox))
    }

    /// Serve a DhtEvent sent to us.
    /// Return a list of P2pProtocol messages for us to process.
    // FIXME
    fn serve_DhtEvent(&mut self, evt: DhtEvent) -> Lib3hResult<(DidWork, Vec<P2pProtocol>)> {
        let mut outbox = Vec::new();
        let mut did_work = false;
        match evt {
            DhtEvent::RemoteGossipBundle(_data) => {
                // FIXME
            }
            DhtEvent::GossipTo(_data) => {
                // FIXME
            }
            DhtEvent::UnreliableGossipTo(_data) => {
                // FIXME
            }
            DhtEvent::PeerHoldRequest(_data) => {
                // FIXME
            }
            DhtEvent::PeerTimedOut(_peer_address) => {
                // FIXME
            }
            DhtEvent::DataHoldRequest(_data) => {
                // FIXME
            }
            DhtEvent::DataFetch(_data) => {
                // FIXME
            }
            DhtEvent::DataFetchResponse(_data) => {
                // FIXME
            }
            DhtEvent::DataPrune(_peer_address) => {
                // FIXME
            }
        }
        Ok((did_work, outbox))
    }
}
