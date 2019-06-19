#![allow(non_snake_case)]

use crate::{
    dht::{
        dht_protocol::*,
        dht_trait::{Dht, DhtConfig, DhtFactory},
    },
    gateway::P2pGateway,
    transport::{
        error::{TransportError, TransportResult},
        transport_trait::Transport,
        TransportIdRef,
    },
};
use lib3h_protocol::{AddressRef, Lib3hResult};
use std::{cell::RefCell, rc::Rc};

/// Public interface
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    /// This Gateways identifier
    pub fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    /// Hack dumb rust compiler
    pub fn post_dht(&mut self, cmd: DhtCommand) -> Lib3hResult<()> {
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
        }
    }
}

/// Private
impl<T: Transport, D: Dht> P2pGateway<T, D> {
    /// Get Transports from the DHT
    pub(crate) fn address_to_transport_list(
        &self,
        id_list: &[&TransportIdRef],
    ) -> TransportResult<Vec<String>> {
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
                Some(peer) => transport_list.push(peer.transport.to_string()),
            }
        }
        Ok(transport_list)
    }
}
