#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht, rrdht::RrDht},
    p2p_protocol::P2pProtocol,
    transport::{
        error::TransportResult,
        memory_mock::transport_memory::TransportMemory,
        protocol::{TransportCommand, TransportEvent},
        transport_trait::Transport,
        TransportId, TransportIdRef,
    },
    transport_wss::TransportWss,
};
use lib3h_protocol::{data_types::EntryData, AddressRef, DidWork, Lib3hResult};


/// Gateway to a P2P network.
/// Enables Connections to many other nodes.
/// Tracks distributed data for that P2P network.
/// P2pGateway should not `post() & process()` its inner transport but call it synchrounously.
/// Composite pattern for Transport and Dht
pub struct P2pGateway<'t, T: Transport, D: Dht> {
    inner_transport: &'t T,
    inner_dht: D,
    maybe_advertise: Option<String>,
}

impl<'t, T: Transport, RrDht> P2pGateway<'t, T, RrDht> {
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new(inner_transport: &T) -> Self {
        P2pGateway {
            inner_transport,
            inner_dht: RrDht::new(),
            maybe_advertise: None,
        }
    }
}

impl P2pGateway<TransportMemory, RrDht> {
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new_with_memory(name: &str) -> Self {
        let mut gateway = P2pGateway {
            inner_transport: TransportMemory::new(),
            inner_dht: RrDht::new(),
            maybe_advertise: None,
        };
        let binding = gateway
            .bind(name)
            .expect("TransportMemory.bind() failed. url/name might not be unique?");
        gateway.maybe_advertise = Some(binding);
        gateway
    }
}

impl P2pGateway<TransportWss<std::net::TcpStream>, RrDht> {
    /// Constructor
    pub fn new_with_wss() -> Self {
        P2pGateway {
            inner_transport: TransportWss::with_std_tcp_stream(),
            inner_dht: RrDht::new(),
            maybe_advertise: None,
        }
    }
}

impl<T: Transport, D: DHT> P2pGateway<P2pGateway<T, D>, RrDht> {
    /// Constructors
    pub fn new_with_space(gateway: T) -> Self {
        P2pGateway {
            inner_transport: TransportSpace::new(recv),
            inner_dht: RrDht::new(),
            maybe_advertise: None,
        }
    }
}

/// Public interface
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    // -- Getters -- //
    /// This nodes identifier on the network
    pub fn id(&self) -> String {
        self.inner_dht
            .this_peer()
            .expect("P2pGateway's DHT should have 'this_peer'")
            .to_string()
    }
    /// This nodes connection address
    pub fn advertise(&self) -> Option<String> {
        self.maybe_advertise.clone()
    }
}

/// Compose DHT
impl<T: Transport, D: Dht> Dht for P2pGateway<T, D> {
    /// Peer info
    fn get_peer(&self, peer_address: &str) -> Option<PeerData> {
        self.inner_dht.get_peer(peer_address)
    }
    fn fetch_peer(&self, peer_address: &str) -> Option<PeerData> {
        self.inner_dht.fetch_peer(peer_address)
    }
    /// Entry
    fn get_entry(&self, entry_address: &AddressRef) -> Option<EntryData> {
        self.inner_dht.get_entry(entry_address)
    }
    fn fetch_entry(&self, entry_address: &AddressRef) -> Option<EntryData> {
        self.inner_dht.fetch_entry(entry_address)
    }
    /// Processing
    fn post(&mut self, cmd: DhtCommand) -> Lib3hResult<()> {
        self.inner_dht.post(cmd)
    }
    /// FIXME: should P2pGateway `post() & process()` its inner dht?
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)> {
        // Process the dht
        let (did_work, dht_event_list) = self.inner_dht.process()?;
        Ok((did_work, dht_event_list))
    }
    /// Getters
    fn this_peer(&self) -> Lib3hResult<&str> {
        self.inner_dht.this_peer()
    }
    fn get_peer_list(&self) -> Vec<PeerData> {
        self.inner_dht.get_peer_list()
    }
}

/// Compose Transport
impl<T: Transport, D: Dht> Transport for P2pGateway<T, D> {
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
        // FIXME: query peer in dht first
        let mut transport_list = Vec::with_capacity(id_list.len);
        for transportId in id_list {
            let maybe_peer = self.inner_dht.get_peer(transportId);
            match maybe_peer {
                None => return Err(format_err!("Unknown transportId: {}", transportId)),
                Some(peer) => transport_list.push(peer.transport.as_str()),
            }
        }
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

    /// FIXME: should P2pGateway `post() & process()` its inner transport?
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        // self.inner_transport.process()
        Ok((false, vec![]))
    }

    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        self.inner_transport.transport_id_list()
    }
}

/// Public - specific
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    pub fn do_process(&mut self) -> Lib3hResult<(DidWork, Vec<P2pProtocol>)> {
        let mut outbox = Vec::new();
//        // Process the transport connection
//        let (did_work, event_list) = self.inner_transport.process()?;
//        if did_work {
//            for evt in event_list {
//                let mut p2p_output = self.handle_TransportEvent(&evt)?;
//                // Add p2p events to outbox
//                outbox.append(&mut p2p_output);
//            }
//        }
        // Process the dht
        let (did_work, dht_event_list) = self.inner_dht.process()?;
        if did_work {
            for evt in dht_event_list {
                let (did_work, mut p2p_output) = self.handle_DhtEvent(evt)?;
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
    /// Process a transportEvent received from our internal connection.
    /// Return a list of P2pProtocol messages to send to others.
    fn handle_TransportEvent(&mut self, evt: &TransportEvent) -> Lib3hResult<Vec<P2pProtocol>> {
        println!("(log.d) >>> '(TransportGateway)' recv: {:?}", evt);
        let mut outbox: Vec<P2pProtocol> = Vec::new();
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
            P2pProtocol::DirectMessage(_) => {
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

    /// Handle a DhtEvent sent to us by our internal DHT.
    /// Return a list of P2pProtocol messages for our owner to process.
    // FIXME
    fn handle_DhtEvent(&mut self, cmd: DhtEvent) -> Lib3hResult<(DidWork, Vec<P2pProtocol>)> {
        let outbox = Vec::new();
        let did_work = false;
        match cmd {
            DhtEvent::GossipTo(_data) => {
                // FIXME: DHT should give us the peer_transport
                // Forward gossip to the inner_transport
                // If no connection to that transportId is open, open one first.
                self.inner_transport.send();
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
            DhtEvent::HoldEntryRequested(_data) => {
                // FIXME
            }
            DhtEvent::FetchEntryResponse(_data) => {
                // FIXME
            }
            DhtEvent::EntryPruned(_address) => {
                // FIXME
            }
        }
        Ok((did_work, outbox))
    }
}
