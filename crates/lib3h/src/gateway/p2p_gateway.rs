#![allow(non_snake_case)]

use crate::{
    dht::{dht_trait::DhtConfig, ghost_protocol::*},

    gateway::{Gateway, P2pGateway},
    transport::{protocol::*, TransportWrapper},
};
use lib3h_protocol::Address;
use std::collections::{HashMap, VecDeque};
use detach::prelude::*;

//--------------------------------------------------------------------------------------------------
// Constructors
//--------------------------------------------------------------------------------------------------

/// any Transport Constructor
impl<'gateway> P2pGateway<'gateway> {
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new(
        identifier: &str,
        inner_transport: TransportWrapper<'gateway>,
        dht_factory: DhtFactory,
        dht_config: &DhtConfig,
    ) -> Self {
        let dht = dht_factory(dht_config).expect("Failed to construct DHT");
        P2pGateway {
            inner_transport,
            inner_dht: Detach::new(DhtParentWrapper::new(dht)),
            identifier: identifier.to_owned(),
            connection_map: HashMap::new(),
            transport_inbox: VecDeque::new(),
            transport_inject_events: Vec::new(),
        }
    }
}

impl<'gateway> Gateway for P2pGateway<'gateway> {
    /// This Gateway's identifier
    fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    fn transport_inject_event(&mut self, evt: TransportEvent) {
        self.transport_inject_events.push(evt);
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

/// P2pGateway Constructor
impl<'gateway> P2pGateway<'gateway> {
    /// Constructors
    pub fn new_with_space(
        network_gateway: TransportWrapper<'gateway>,
        space_address: &Address,
        dht_factory: DhtFactory,
        dht_config: &DhtConfig,
    ) -> Self {
        let identifier: String = space_address.clone().into();
        P2pGateway {
            inner_transport: network_gateway,
            inner_dht: dht_factory(dht_config).expect("Failed to construct DHT"),
            identifier,
            connection_map: HashMap::new(),
            transport_inbox: VecDeque::new(),
            transport_inject_events: Vec::new(),
        }
    }
}
