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
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

/// Public interface
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    /// This Gateway's identifier
    pub fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    /// Hack: explicit post because of dumb rust compiler
    pub fn post_dht(&mut self, cmd: DhtCommand) -> Lib3hResult<()> {
        // HACK: Add fake reverse on HoldPeer
        if let DhtCommand::HoldPeer(peer_data) = cmd.clone() {
            // println!("ADDIND FAKE REVERSE: {}", peer_data.transport.clone());
            self.connection_map
                .insert(peer_data.transport.clone(), peer_data.transport.clone());
        }
        // HACK END
        self.inner_dht.post(cmd)
    }
}

//--------------------------------------------------------------------------------------------------
// Constructors
//--------------------------------------------------------------------------------------------------

/// any Transport Constructor
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
            connection_map: HashMap::new(),
            transport_inbox: VecDeque::new(),
        }
    }
}

/// P2pGateway Constructor
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
            connection_map: HashMap::new(),
            transport_inbox: VecDeque::new(),
        }
    }
}
