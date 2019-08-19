#![allow(non_snake_case)]

use crate::{
    dht::dht_trait::{Dht, DhtConfig, DhtFactory},
    gateway::{Gateway, P2pGateway},
    transport::protocol::*,
};
use lib3h_protocol::Address;
use std::collections::{HashSet, VecDeque};
use url::Url;

//--------------------------------------------------------------------------------------------------
// Constructors
//--------------------------------------------------------------------------------------------------

/// any Transport Constructor
impl<'gateway, D: Dht> P2pGateway<'gateway, D> {
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new(
        address_url_scheme: &str,
        space_address: Address,
        identifier: &str,
        //inner_transport: TransportWrapper<'gateway>,
        dht_factory: DhtFactory<D>,
        dht_config: &DhtConfig,
    ) -> Self {
        P2pGateway {
            address_url_scheme: address_url_scheme.to_string(),
            space_address,
            //inner_transport,
            inner_dht: dht_factory(dht_config).expect("Failed to construct DHT"),
            identifier: identifier.to_owned(),
            connections: HashSet::new(),
            transport_inbox: VecDeque::new(),
            transport_injected_events: Vec::new(),
            transport_sends: Vec::new(),
            phantom_data: std::marker::PhantomData,
        }
    }
}

impl<'gateway, D: Dht> Gateway for P2pGateway<'gateway, D> {
    /// This Gateway's identifier
    fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    fn inject_transport_event(&mut self, evt: TransportEvent) {
        self.transport_injected_events.push(evt);
    }

    fn drain_transport_sends(&mut self) -> Vec<(String, Url, Vec<u8>)> {
        self.transport_sends.drain(..).collect()
    }
}
