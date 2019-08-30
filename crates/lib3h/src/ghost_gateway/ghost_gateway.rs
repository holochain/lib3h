use crate::{
    dht::{
        dht_protocol::PeerData,
        dht_trait::{Dht, DhtConfig, DhtFactory},
    },
    ghost_gateway::GhostGateway,
    transport::{error::TransportError, protocol::*},
};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::Address;

impl<'gateway, D: Dht> GhostGateway<D> {
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new(
        identifier: &str,
        inner_transport: impl GhostActor<
            TransportRequestToParent,
            TransportRequestToParentResponse,
            TransportRequestToChild,
            TransportRequestToChildResponse,
            TransportError,
        >,
        dht_factory: DhtFactory<D>,
        dht_config: &DhtConfig,
    ) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let child_transport = Detach::new(GhostParentWrapper::new(
            Box::new(inner_transport),
            "to_child_transport",
        ));
        GhostGateway {
            endpoint_parent: Some(endpoint_parent),
            endpoint_self: Some(endpoint_self.as_context_endpoint("from_gateway_parent")),
            child_transport,
            inner_dht: dht_factory(dht_config).expect("Failed to construct DHT"),
            identifier: identifier.to_owned(),
        }
    }

    ///
    pub fn new_with_space(
        network_gateway: Detach<TransportParentWrapper>,
        space_address: &Address,
        dht_factory: DhtFactory<D>,
        dht_config: &DhtConfig,
    ) -> Self {
        let identifier: String = space_address.clone().into();
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        GhostGateway {
            endpoint_parent: Some(endpoint_parent),
            endpoint_self: Some(endpoint_self.as_context_endpoint("from_gateway_parent")),
            child_transport: network_gateway,
            inner_dht: dht_factory(dht_config).expect("Failed to construct DHT"),
            identifier,
        }
    }

    pub fn this_peer(&self) -> &PeerData {
        self.inner_dht.this_peer()
    }
}
