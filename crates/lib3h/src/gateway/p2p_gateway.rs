#![allow(non_snake_case)]

use crate::{
    track::Tracker,
    dht::dht_trait::{Dht, DhtConfig, DhtFactory},
    gateway::P2pGateway,
    transport::transport_trait::Transport,
};
use lib3h_protocol::Address;
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

    pub fn process(&mut self) -> OENTUHNOTEHUNTHENOU {
        //putting this here so we don't forget

        for (timeout_id, timeout_data) in self.request_track.process_timeouts() {
            error!("timeout {:?} {:?}", timeout_id, timeout_data);
        }
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
            request_track: Tracker::new("gateway_", 2000),
            connection_map: HashMap::new(),
            transport_inbox: VecDeque::new(),
        }
    }

    /// Helper for getting a connectionId from a peer_address
    pub(crate) fn get_connection_id(&self, peer_address: &str) -> Option<String> {
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

/// P2pGateway Constructor
impl<T: Transport, D: Dht> P2pGateway<P2pGateway<T, D>, D> {
    /// Constructors
    pub fn new_with_space(
        network_gateway: Rc<RefCell<P2pGateway<T, D>>>,
        space_address: &Address,
        dht_factory: DhtFactory<D>,
        dht_config: &DhtConfig,
    ) -> Self {
        let identifier: String = space_address.clone().into();
        P2pGateway {
            inner_transport: network_gateway,
            inner_dht: dht_factory(dht_config).expect("Failed to construct DHT"),
            identifier,
            request_track: Tracker::new("gateway_", 2000),
            connection_map: HashMap::new(),
            transport_inbox: VecDeque::new(),
        }
    }
}
