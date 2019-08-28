use lib3h_ghost_actor::prelude::*;
use crate::{
    dht::dht_trait::{Dht, DhtConfig, DhtFactory},
    transport::{
        TransportWrapper,
        protocol::*,
        error::{TransportError, TransportResult},
    },
    gateway::Gateway,
    ghost_gateway::GhostGateway,
};
use detach::prelude::*;

impl<'gateway, D: Dht> GhostGateway<D> {
    #[allow(dead_code)]
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new(
        identifier: &str,
        inner_transport: TransportWrapper<'gateway>,
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
            endpoint_self: Some(endpoint_self.as_context_channel("from_gateway_parent")),
            child_transport,
            inner_dht: dht_factory(dht_config).expect("Failed to construct DHT"),
            identifier: identifier.to_owned(),
        }
    }
}
