use crate::{
    dht::{dht_config::DhtConfig, dht_protocol::*},
    engine::GatewayId,
    gateway::{GatewayOutputWrapType, P2pGateway},
    message_encoding::*,
    transport,
};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::uri::{Lib3hUri, UriScheme};

//--------------------------------------------------------------------------------------------------
// Constructors
//--------------------------------------------------------------------------------------------------

impl P2pGateway {
    pub fn new(
        wrap_output_type: GatewayOutputWrapType,
        identifier: GatewayId,
        this_peer_location: Lib3hUri,
        inner_transport: transport::protocol::DynTransportActor,
        dht_factory: DhtFactory,
        dht_config: &DhtConfig,
    ) -> Self {
        // Create this_peer
        let this_peer = PeerData {
            peer_name: dht_config.this_peer_name(),
            peer_location: this_peer_location.clone(),
            timestamp: crate::time::since_epoch_ms(),
        };
        let maybe_this_peer = if this_peer_location.is_scheme(UriScheme::Undefined) {
            None
        } else {
            Some(this_peer.clone())
        };
        // Create dht actor
        let dht = dht_factory(dht_config, maybe_this_peer).expect("Failed to construct DHT");
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_self = Detach::new(
            endpoint_self
                .as_context_endpoint_builder()
                .request_id_prefix(&format!("{}_to_parent_", identifier.nickname))
                .build(),
        );
        // create gateway
        P2pGateway {
            wrap_output_type,
            identifier: identifier,
            inner_transport: Detach::new(transport::protocol::TransportActorParentWrapperDyn::new(
                inner_transport,
                "to_child_transport_",
            )),
            inner_dht: Detach::new(ChildDhtWrapperDyn::new(dht, "gateway_dht_")),
            message_encoding: Detach::new(GhostParentWrapper::new(
                MessageEncoding::new(),
                "to_message_encoding_",
            )),
            endpoint_parent: Some(endpoint_parent),
            endpoint_self,
            this_peer,
            pending_send_queue: Vec::new(),
        }
    }

    pub fn this_peer(&self) -> PeerData {
        self.this_peer.clone()
    }

    /// Retrieve the list of peer from a dht.
    /// TODO: Find a way to actually retrive this list from an async call :)
    pub fn get_peer_list(&mut self) -> Vec<String> {
        // pub fn get_peer_list(&mut self) -> Vec<PeerData> {
        let inner_dht = &mut self.inner_dht;
        inner_dht
            .request(
                holochain_tracing::Span::fixme(),
                crate::dht::dht_protocol::DhtRequestToChild::RequestPeerList,
                Box::new(|_, r| {
                    eprintln!("1 got: {:?}", r);
                    Ok(())
                }),
            )
            .unwrap();

        vec!["TODO: get_peer_list's result".to_string()]
    }
}
