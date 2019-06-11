#![allow(non_snake_case)]

use crate::{
    dht::{
        dht_event::{DhtEvent, PeerHoldRequestData},
        dht_trait::Dht,
        rrdht::RrDht,
    },
    p2p::p2p_protocol::P2pProtocol,
    transport::{
        error::TransportResult,
        memory_mock::transport_memory::TransportMemory,
        protocol::{TransportCommand, TransportEvent},
        transport_trait::Transport,
        TransportId, TransportIdRef,
    },
    transport_space::TransportSpace,
    transport_wss::TransportWss,
};
use lib3h_protocol::{AddressRef, DidWork, Lib3hResult};

/// Gateway to a P2P network.
/// Enables Connections to many other nodes.
/// Tracks distributed data for that P2P network.
pub struct P2pGateway<T: Transport, D: Dht> {
    connection: T,
    dht: D,
    maybe_advertise: Option<String>,
}

impl P2pGateway<TransportMemory, RrDht> {
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new_with_memory(name: &str) -> Self {
        let mut gateway = P2pGateway {
            connection: TransportMemory::new(),
            dht: RrDht::new(),
            maybe_advertise: None,
        };
        let binding = gateway
            .bind(name)
            .expect("TransportMemory.bind() failed. url/name might not be unique?");
        gateway.maybe_advertise = Some(binding);
        gateway
    }
}

impl<D: Dht> P2pGateway<TransportWss<std::net::TcpStream>, D> {
    /// Constructor
    pub fn new_with_wss() -> Self {
        P2pGateway {
            connection: TransportWss::with_std_tcp_stream(),
            dht: RrDht::new(),
            maybe_advertise: None,
        }
    }
}

impl<D: Dht> P2pGateway<TransportSpace, D> {
    /// Constructor
    pub fn new_with_space() -> Self {
        P2pGateway {
            connection: TransportSpace::new(),
            dht: RrDht::new(),
            maybe_advertise: None,
        }
    }
}

/// Public interface
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    // -- Getters -- //
    /// This nodes identifier on the network
    pub fn id(&self) -> String {
        self.dht.this_peer().expect("P2pGateway's DHT should have 'this_peer'")
    }
    /// This nodes connection address
    pub fn advertise(&self) -> Option<String> {
        self.maybe_advertise
    }
}

/// Compose DHT
impl<T: Transport, D: Dht> Dht for P2pGateway<T, D> {
    /// Peer info
    fn get_peer(&self, peer_address: &str) -> Option<PeerHoldRequestData> {
        self.dht.get_peer(peer_address)
    }
    fn fetch_peer(&self, peer_address: &str) -> Option<PeerHoldRequestData> {
        self.dht.fetch_peer(peer_address)
    }
    fn drop_peer(&self, peer_address: &str) -> Lib3hResult<()> {
        self.dht.drop_peer(peer_address)
    }
    /// Data
    fn get_data(&self, data_address: &AddressRef) -> Lib3hResult<Vec<u8>> {
        self.dht.get_data(data_address)
    }
    fn fetch_data(&self, data_address: &AddressRef) -> Lib3hResult<Vec<u8>> {
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
impl<T: Transport, D: Dht> Transport for P2pGateway<T, D> {
    fn connect(&mut self, uri: &str) -> TransportResult<TransportId> {
        self.connection.connect(&uri)
    }
    fn close(&mut self, id: &TransportIdRef) -> TransportResult<()> {
        self.connection.close(id)
    }

    fn close_all(&mut self) -> TransportResult<()> {
        self.connection.close_all()
    }

    fn send(&mut self, id_list: &[&TransportIdRef], payload: &[u8]) -> TransportResult<()> {
        self.connection.send(id_list, payload)
    }

    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        self.connection.send_all(payload)
    }

    fn bind(&mut self, url: &str) -> TransportResult<String> {
        let res = self.connection.bind(url);
        if let Some(adv) = res {
            self.maybe_advertise = Some(adv);
        }
        res
    }

    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.connection.post(command)
    }

    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        self.connection.process()
    }

    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        self.connection.transport_id_list()
    }
}

/// Public - specific
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    pub fn do_process(&mut self) -> Lib3hResult<(DidWork, Vec<P2pProtocol>)> {
        let mut outbox = Vec::new();
        // Process the transport connection
        let (did_work, event_list) = self.connection.process()?;
        if did_work {
            for evt in event_list {
                let mut p2p_output = self.serve_TransportEvent(&evt)?;
                // Add p2p events to outbox
                outbox.append(&mut p2p_output);
            }
        }
        // Process the dht
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
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    /// Process a transportEvent.
    /// Return a list of P2pProtocol messages to send to others.
    fn serve_TransportEvent(&mut self, evt: &TransportEvent) -> Lib3hResult<Vec<P2pProtocol>> {
        println!("(log.d) >>> '(TransportGateway)' recv: {:?}", evt);
        let mut outbox: Vec<P2pProtocol> = Vec::new();
        // Note: use same order as the enum
        match evt {
            TransportEvent::TransportError(id, e) => {
                println!(
                    "(log.e) Connection Error for {}: {}\n Closing connection.",
                    id, e
                );
                self.connection.close(id)?;
            }
            TransportEvent::ConnectResult(id) => {
                // don't need to do anything here
                println!("(log.i) Connection opened: {}", id);
            }
            TransportEvent::Closed(id) => {
                // FIXME
                println!("(log.w) Connection closed: {}", id);
                self.connection.close(id)?;
                //let _transport_id = self.wss_socket.wait_connect(&self.ipc_uri)?;
            }
            TransportEvent::Received(id, msg) => {
                println!("(log.d) Received message from: {}", id);
                // FIXME: Make sense of msg?
                let p2p_msg = match P2pProtocol::deserialize(msg) {
                    Err(e) => {
                        println!("(log.e) Payload failed to deserialize: {:?}", msg);
                        return Err(e);
                    }
                    Ok(obj) => obj,
                };
                outbox = self.serve_P2pProtocol(&p2p_msg)?;
            }
        };
        Ok(outbox)
    }

    /// Serve a P2pProtocol sent to us.
    /// Handle it or pass it along.
    /// Return a list of P2pProtocol messages for others to process.
    // FIXME
    fn serve_P2pProtocol(&mut self, p2p_msg: &P2pProtocol) -> Lib3hResult<Vec<P2pProtocol>> {
        let outbox = Vec::new();
        match p2p_msg {
            P2pProtocol::Gossip => {
                // FIXME
            }
            P2pProtocol::DirectMessage => {
                // FIXME
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

    /// Serve a DhtEvent sent to us by our internal DHT.
    /// Return a list of P2pProtocol messages for our owner to process.
    // FIXME
    fn serve_DhtEvent(&mut self, evt: DhtEvent) -> Lib3hResult<(DidWork, Vec<P2pProtocol>)> {
        let outbox = Vec::new();
        let did_work = false;
        match evt {
            DhtEvent::RemoteGossipBundle(_data) => {
                // FIXME: Ask our owner to gossip data back?
            }
            DhtEvent::GossipTo(_data) => {
                // FIXME: Ask our owner to gossip data
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
