#![allow(non_snake_case)]

use crate::{
    dht::{
        dht_protocol::*,
        dht_trait::{Dht, DhtConfig, DhtFactory},
    },
    gateway::P2pGateway,
    transport::transport_trait::Transport,
};
use lib3h_protocol::{AddressRef, Lib3hResult};
use std::{cell::RefCell, rc::Rc};

/// Public interface
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    //    // -- Getters -- //
    //    /// This nodes identifier on the network
    //    pub fn id(&self) -> String {
    //        self.inner_dht
    //            .this_peer()
    //            .expect("P2pGateway's DHT should have 'this_peer'")
    //            .to_string()
    //    }

    /// This Gateways identifier
    pub fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    /// Hack dumb rust compiler
    pub fn post_dht(&mut self, cmd: DhtCommand) -> Lib3hResult<()> {
        self.inner_dht.post(cmd)
    }

    //    pub fn set_advertise(&mut self, binding: &str) {
    //        self.maybe_advertise = Some(binding.to_string());
    //    }
}

//--------------------------------------------------------------------------------------------------
// Constructors
//--------------------------------------------------------------------------------------------------

/// any Transport
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new(
        identifier: &str,
        inner_transport: Rc<RefCell<T>>,
        dht_factory: DhtFactory<D>,
        dht_config: &DhtConfig,
    ) -> Self {
        P2pGateway {
            inner_transport,
            inner_dht: dht_factory(dht_config).expect("Failed to construct DHT"),
            identifier: identifier.to_owned(),
        }
    }
}

/// P2pGateway
impl<T: Transport, D: Dht> P2pGateway<P2pGateway<T, D>, D> {
    /// Constructors
    pub fn new_with_space(
        network_gateway: Rc<RefCell<P2pGateway<T, D>>>,
        space_address: &AddressRef,
        dht_factory: DhtFactory<D>,
        dht_config: &DhtConfig,
    ) -> Self {
        let identifier = std::string::String::from_utf8_lossy(space_address).to_string();
        P2pGateway {
            inner_transport: network_gateway,
            inner_dht: dht_factory(dht_config).expect("Failed to construct DHT"),
            identifier,
        }
    }
}
