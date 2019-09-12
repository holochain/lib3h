use crate::{
    dht::{dht_config::DhtConfig, dht_protocol::*},
    gateway::P2pGateway,
    transport,
};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::Address;
use lib3h_tracing::Lib3hTrace;

//--------------------------------------------------------------------------------------------------
// Constructors
//--------------------------------------------------------------------------------------------------

/// P2pGateway Constructors
impl P2pGateway {
    /// Constructor
    /// Bind and set advertise on construction by using the name as URL.
    pub fn new(
        identifier: &str,
        child_transport_endpoint: Detach<
            transport::protocol::TransportActorParentContextEndpoint<P2pGateway, Lib3hTrace>,
        >,
        dht_factory: DhtFactory,
        dht_config: &DhtConfig,
    ) -> Self {
        let dht = dht_factory(dht_config).expect("Failed to construct DHT");
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_self = Detach::new(
            endpoint_self
                .as_context_endpoint_builder()
                .request_id_prefix(&format!("{}_to_parent_", identifier))
                .build(),
        );
        P2pGateway {
            identifier: identifier.to_owned(),
            child_transport_endpoint,
            inner_dht: Detach::new(ChildDhtWrapperDyn::new(dht, "gateway_dht")),
            endpoint_parent: Some(endpoint_parent),
            endpoint_self,
        }
    }
    /// Helper Ctor
    pub fn new_with_space(
        space_address: &Address,
        child_transport_endpoint: Detach<
            transport::protocol::TransportActorParentContextEndpoint<P2pGateway, Lib3hTrace>,
        >,
        dht_factory: DhtFactory,
        dht_config: &DhtConfig,
    ) -> Self {
        let identifier: String = space_address.clone().into();
        P2pGateway::new(
            &identifier,
            child_transport_endpoint,
            dht_factory,
            dht_config,
        )
    }

    pub fn this_peer(&self) -> PeerData {
        // self.inner_dht().as_mut().as_mut()
        PeerData {
            peer_address: "FIXME".to_string(),
            peer_uri: Url::parse("fixme://host:123").unwrap(),
            timestamp: 0,
        }
    }
}
//
//impl P2pGateway {
//    // FIXME
//    pub fn drain_dht_outbox(&mut self) -> Vec<Lib3hServerProtocol> {
//        self.user_data.lib3h_outbox.drain(0..).collect()
//    }
//}
