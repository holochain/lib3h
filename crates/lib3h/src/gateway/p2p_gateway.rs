#![allow(non_snake_case)]

use crate::{
    dht::dht_trait::{Dht, DhtConfig, DhtFactory},
    gateway::{Gateway, P2pGateway},
    transport::TransportWrapper,
};
use std::collections::{HashSet, VecDeque};

//--------------------------------------------------------------------------------------------------
// Constructors
//--------------------------------------------------------------------------------------------------

/// any Transport Constructor
impl<'gateway, D: Dht> P2pGateway<'gateway, D> {
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new(
        identifier: &str,
        inner_transport: TransportWrapper<'gateway>,
        dht_factory: DhtFactory<D>,
        dht_config: &DhtConfig,
    ) -> Self {
        P2pGateway {
            inner_transport,
            inner_dht: dht_factory(dht_config).expect("Failed to construct DHT"),
            identifier: identifier.to_owned(),
            connections: HashSet::new(),
            transport_inbox: VecDeque::new(),
        }
    }
}

impl<'gateway, D: Dht> Gateway for P2pGateway<'gateway, D> {
    /// This Gateway's identifier
    fn identifier(&self) -> &str {
        self.identifier.as_str()
    }
}
