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
    pub(crate) inner_transport: &'t T,
    pub(crate) inner_dht: D,
    ///
    pub(crate) maybe_advertise: Option<String>,
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
    pub fn new_with_space(network_gateway: &P2pGateway<T, D>, space_address: &AddressRef) -> Self {
        P2pGateway {
            inner_transport: network_gateway,
            inner_dht: RrDht::new(),
            maybe_advertise: Some(space_address.to_string()),
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



///// Public - specific
//impl<T: Transport, D: Dht> P2pGateway<T, D> {
//    pub fn do_process(&mut self) -> Lib3hResult<(DidWork, Vec<P2pProtocol>)> {
//        let mut outbox = Vec::new();
////        // Process the transport connection
////        let (did_work, event_list) = self.inner_transport.process()?;
////        if did_work {
////            for evt in event_list {
////                let mut p2p_output = self.handle_TransportEvent(&evt)?;
////                // Add p2p events to outbox
////                outbox.append(&mut p2p_output);
////            }
////        }
//        // Process the dht
//        let (did_work, dht_event_list) = self.inner_dht.process()?;
//        if did_work {
//            for evt in dht_event_list {
//                self.handle_DhtEvent(evt)?;
//            }
//        }
//        // Done
//        Ok((did_work, outbox))
//    }
//}

///// Private internals
//impl<T: Transport, D: Dht> P2pGateway<T, D> {
//    /// Serve a P2pProtocol sent to us.
//    /// Handle it or pass it along.
//    /// Return a list of P2pProtocol messages for others to process.
//    // FIXME
//    fn serve_P2pProtocol(&mut self, p2p_msg: &P2pProtocol) -> Lib3hResult<Vec<P2pProtocol>> {
//        let outbox = Vec::new();
//        match p2p_msg {
//            P2pProtocol::Gossip(_) => {
//                // FIXME
//            }
//            P2pProtocol::DirectMessage(_) => {
//                // FIXME
//            }
//            P2pProtocol::FetchData => {
//                // FIXME
//            }
//            P2pProtocol::FetchDataResponse => {
//                // FIXME
//            }
//        };
//        Ok(outbox)
//    }
//}
