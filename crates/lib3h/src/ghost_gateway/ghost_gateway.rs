use lib3h_ghost_actor::{
    prelude::*,
    ghost_channel::*,
};
use crate::transport::memory_mock::ghost_transport_memory::*;

impl<'gateway, D: Dht> GhostGateway<'gateway, D> {
    #[allow(dead_code)]
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new(
        identifier: &str,
        inner_transport: TransportWrapper<'gateway>,
        dht_factory: DhtFactory<D>,
        dht_config: &DhtConfig,
    ) -> Self {
        let (endpoint_parent, endpoint_self) = ghost_channel::create_ghost_channel<
            TransportRequestToParent,
            TransportRequestToParentResponse,
            TransportRequestToChild,
            TransportRequestToChildResponse,
            TransportError,
        >();
        let child_transport = Detach::new(GhostParentContextChannel::new(
            Box::new(inner_transport),
            "to_child_transport",
        ));
        GhostGateway {
            endpoint_parent: Some(endpoint_parent),
            endpoint_self: Some(endpoint_self.as_context_channel("from_gateway_parent")),
            child_transport,
            inner_dht: dht_factory(dht_config).expect("Failed to construct DHT"),
            identifier: identifier.to_owned(),
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Gateway trait
//--------------------------------------------------------------------------------------------------

impl<'gateway, D: Dht> Gateway for GhostGateway<'gateway, D> {
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
        Some(peer_uri.to_string())
    }
}
