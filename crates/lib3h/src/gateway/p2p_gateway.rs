#![allow(non_snake_case)]

use crate::{
    dht::dht_trait::{Dht, DhtConfig, DhtFactory},
    error::Lib3hResult,
    gateway::{Gateway, P2pGateway, TrackType},
    track::Tracker,
    transport::TransportWrapper,
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
        identifier: &str,
        inner_transport: TransportWrapper<'gateway>,
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
            transport_outbox: Vec::new(),
            workflow: Vec::new(),
        }
    }

    /// register a followup tracker id
    pub(crate) fn register_track(&mut self, user_data: TrackType) -> String {
        let id = self.request_track.gen_id();
        if let Some(_) = self.request_track.set(&id, Some(user_data.clone())) {
            panic!("unexpected id already used!! {} {:?}", id, user_data);
        }
        id
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

    fn process(&mut self) -> Lib3hResult<()> {
        for (timeout_id, timeout_data) in self.request_track.process_timeouts() {
            error!("timeout {:?} {:?}", timeout_id, timeout_data);
        }
        Ok(())
    }
}

/// P2pGateway Constructor
impl<'gateway, D: Dht> P2pGateway<'gateway, D> {
    /// Constructors
    pub fn new_with_space(
        network_gateway: TransportWrapper<'gateway>,
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
            transport_outbox: Vec::new(),
            workflow: Vec::new(),
        }
    }
}
