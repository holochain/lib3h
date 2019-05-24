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
        protocol::{TransportCommand, TransportEvent},
        transport_trait::Transport,
        TransportId, TransportIdRef,
    },
    transport_dna::TransportDna,
    transport_wss::TransportWss,
};
use holochain_lib3h_protocol::{Address, DidWork, Lib3hResult};

/// Gateway to a P2P network.
/// Enables Connections to many other nodes.
/// Tracks distributed data for that P2P network.
pub struct P2pGateway {
    transport: Box<Transport>,
    dht: Box<Dht>,
}

/// Public interface
impl P2pGateway {
    /// Constructor
    pub fn new(is_dna: bool) -> Self {
        let transport: Box<Transport> = if is_dna {
            Box::new(TransportDna::new())
        //Box::new(TransportWss::with_std_tcp_stream())
        } else {
            Box::new(TransportWss::with_std_tcp_stream())
        };
        P2pGateway {
            transport,
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
        // self.transport_connection.transport_id_list()
        Ok(vec![])
    }

    fn connect(&mut self, uri: &str) -> TransportResult<TransportId> {
        self.transport.connect(&uri)
    }
    fn close(&mut self, id: TransportId) -> TransportResult<()> {
        self.transport.close(id)
    }

    fn close_all(&mut self) -> TransportResult<()> {
        self.transport.close_all()
    }

    fn send(&mut self, id_list: &[&TransportIdRef], payload: &[u8]) -> TransportResult<()> {
        self.transport.send(id_list, payload)
    }

    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        self.transport.send_all(payload)
    }

    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.transport.post(command)
    }

    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        self.transport.process()
    }
}

/// Public - specific
impl P2pGateway {
    pub fn do_process(&mut self) -> Lib3hResult<(DidWork, Vec<P2pProtocol>)> {
        let mut outbox = Vec::new();
        // Process the transport connection
        let (did_work, event_list) = self.transport.process()?;
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
impl P2pGateway {
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
                self.transport.close(id.to_string())?;
            }
            TransportEvent::ConnectResult(id) => {
                // don't need to do anything here
                println!("(log.i) Connection opened: {}", id);
            }
            TransportEvent::Closed(id) => {
                // FIXME
                println!("(log.w) Connection closed: {}", id);
                self.transport.close(id.to_string())?;
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
        let mut outbox = Vec::new();
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
