#![allow(non_snake_case)]

use crate::{
    dht::dht_trait::{Dht, DhtConfig, DhtFactory},
    gateway::{Gateway, P2pGateway},
    transport::{protocol::TransportEvent, TransportWrapper},
};
use lib3h_protocol::Address;
use std::collections::{HashMap, VecDeque};

//--------------------------------------------------------------------------------------------------
// Constructors
//--------------------------------------------------------------------------------------------------

/// any Transport Constructor
impl<'gateway, D: Dht> P2pGateway<'gateway, D> {
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new(
        space_address: Address,
        identifier: &str,
        inner_transport: TransportWrapper<'gateway>,
        dht_factory: DhtFactory<D>,
        dht_config: &DhtConfig,
    ) -> Self {
        P2pGateway {
            space_address,
            transport_outbox: HashMap::new(),
            inner_transport,
            inner_dht: dht_factory(dht_config).expect("Failed to construct DHT"),
            identifier: identifier.to_owned(),
            connection_map: HashMap::new(),
            transport_inbox: VecDeque::new(),
        }
    }
}

impl<'gateway, D: Dht> Gateway for P2pGateway<'gateway, D> {
    /// This Gateway's identifier
    fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    /// Helper for getting a connectionId from a peer_address
    fn get_connection_id(&self, peer_address: &str) -> Option<String> {
        // get peer_uri
        let maybe_peer_data = self.inner_dht.get_peer(peer_address);
        if maybe_peer_data.is_none() {
            return None;
        }
        let peer_uri = maybe_peer_data.unwrap().peer_uri;
        trace!(
            "({}) get_connection_id: {} -> {}",
            self.identifier,
            peer_address,
            peer_uri,
        );
        // get connection_id
        let maybe_connection_id = self.connection_map.get(&peer_uri);
        if maybe_connection_id.is_none() {
            return None;
        }
        let conn_id = maybe_connection_id.unwrap().clone();
        trace!(
            "({}) get_connection_id: {} -> {} -> {}",
            self.identifier,
            peer_address,
            peer_uri,
            conn_id,
        );
        Some(conn_id)
    }
}
