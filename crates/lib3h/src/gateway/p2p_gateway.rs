#![allow(non_snake_case)]

use crate::{
    dht::{dht_protocol::*, dht_trait::Dht, rrdht::RrDht},
    gateway::P2pGateway,
    transport::transport_trait::Transport,
};
use lib3h_protocol::{AddressRef, Lib3hResult};
use std::{cell::RefCell, rc::Rc};

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
impl<T: Transport> P2pGateway<T, RrDht> {
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    // pub fn new(inner_transport: &'t mut T) -> Self {
    pub fn new(inner_transport: Rc<RefCell<T>>) -> Self {
        P2pGateway {
            inner_transport,
            inner_dht: RrDht::new(),
            maybe_advertise: None,
        }
    }
}

/// P2pGateway
impl<T: Transport, D: Dht> P2pGateway<P2pGateway<T, D>, RrDht> {
    /// Constructors
    pub fn new_with_space(
        // network_gateway: &'t mut P2pGateway<'t, T, D>,
        network_gateway: Rc<RefCell<P2pGateway<T, D>>>,
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
