#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht, rrdht::RrDht},
    engine::p2p_protocol::P2pProtocol,
    gateway::{gateway_dht, gateway_transport, P2pGateway},
    transport::{
        error::{TransportError, TransportResult},
        memory_mock::transport_memory::TransportMemory,
        protocol::{TransportCommand, TransportEvent},
        transport_trait::Transport,
        TransportId, TransportIdRef,
    },
    transport_wss::TransportWss,
};
use lib3h_protocol::{data_types::EntryData, AddressRef, DidWork, Lib3hResult};

/// Public interface
impl<'t, T: Transport, D: Dht> P2pGateway<'t, T, D> {
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

    /// Hack dumb rust compiler
    pub fn post_dht(&mut self, cmd: DhtCommand) -> Lib3hResult<()> {
        self.inner_dht.post(cmd)
    }

    pub fn set_advertise(&mut self, binding: &str) {
        self.maybe_advertise = Some(binding.to_string());
    }
}

//--------------------------------------------------------------------------------------------------
// Constructors
//--------------------------------------------------------------------------------------------------

/// any Transport
impl<'t, T: Transport> P2pGateway<'t, T, RrDht> {
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new(inner_transport: &'t T) -> Self {
        P2pGateway {
            inner_transport,
            inner_dht: RrDht::new(),
            maybe_advertise: None,
        }
    }
}

///// TransportMemory
//impl<'t> P2pGateway<'t, TransportMemory, RrDht> {
//    /// Constructor
//    /// Bind and set advertise on construction by using the name as URL.
//    pub fn new_with_memory(name: &str) -> Self {
//        let mut gateway = P2pGateway {
//            inner_transport: &TransportMemory::new(),
//            inner_dht: RrDht::new(),
//            maybe_advertise: None,
//        };
//        let binding = gateway
//            .bind(name)
//            .expect("TransportMemory.bind() failed. url/name might not be unique?");
//        gateway.maybe_advertise = Some(binding);
//        gateway
//    }
//}

///// TransportWss
//impl<'t> P2pGateway<'t, TransportWss<std::net::TcpStream>, RrDht> {
//    /// Constructor
//    pub fn new_with_wss() -> Self {
//        P2pGateway {
//            inner_transport: &TransportWss::with_std_tcp_stream(),
//            inner_dht: RrDht::new(),
//            maybe_advertise: None,
//        }
//    }
//}

/// P2pGateway
impl<'t, T: Transport, D: Dht> P2pGateway<'t, P2pGateway<'t, T, D>, RrDht> {
    /// Constructors
    pub fn new_with_space(
        network_gateway: &'t P2pGateway<'t, T, D>,
        // dht: &'t D,
        space_address: &AddressRef,
    ) -> Self {
        let advertise = std::string::String::from_utf8_lossy(space_address).to_string();
        P2pGateway {
            inner_transport: network_gateway,
            inner_dht: RrDht::new(),
            maybe_advertise: Some(advertise),
        }
    }
}
