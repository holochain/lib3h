#![allow(non_snake_case)]

use crate::{
    dht::{dht_config::DhtConfig, dht_protocol::*},
    gateway::{GatewayUserData, P2pGateway},
    transport,
};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{protocol_server::Lib3hServerProtocol, Address};
use lib3h_tracing::Lib3hSpan;

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
            transport::protocol::TransportActorParentContextEndpoint<GatewayUserData>,
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
            inner_dht: ChildDhtWrapperDyn::new(dht, "gateway_dht"),
            this_peer: PeerData {
                peer_address: dht_config.this_peer_address(),
                peer_uri: dht_config.this_peer_uri(),
                timestamp: 0, // FIXME
            },
            user_data: GatewayUserData::new(),
            endpoint_parent: Some(endpoint_parent),
            endpoint_self,
        }
    }
    /// Helper Ctor
    pub fn new_with_space(
        space_address: &Address,
        child_transport_endpoint: Detach<
            transport::protocol::TransportActorParentContextEndpoint<GatewayUserData>,
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
}

impl P2pGateway {
    // FIXME
    pub fn drain_dht_outbox(&mut self) -> Vec<Lib3hServerProtocol> {
        self.user_data.lib3h_outbox.drain(0..).collect()
    }

    ///
    pub fn get_peer_list_sync(&mut self) -> Vec<PeerData> {
        trace!("get_peer_list_sync() ...");
        self.inner_dht
            .request(
                Lib3hSpan::todo(),
                DhtRequestToChild::RequestPeerList,
                Box::new(|mut ud, response| {
                    let response = {
                        match response {
                            GhostCallbackData::Timeout => panic!("timeout"),
                            GhostCallbackData::Response(response) => match response {
                                Err(e) => panic!("{:?}", e),
                                Ok(response) => response,
                            },
                        }
                    };
                    if let DhtRequestToChildResponse::RequestPeerList(peer_list_response) = response
                    {
                        ud.peer_list = peer_list_response;
                    } else {
                        panic!("bad response to bind: {:?}", response);
                    }
                    Ok(())
                }),
            )
            .expect("sync functions should work");
        self.inner_dht.process(&mut self.user_data).unwrap(); // FIXME unwrap
        self.user_data.peer_list.clone()
    }

    ///
    pub fn get_this_peer_sync(&mut self) -> PeerData {
        // get cached value first
        if self.this_peer.peer_address != String::new() {
            return self.this_peer.clone();
        }
        self.inner_dht
            .request(
                Lib3hSpan::todo(),
                DhtRequestToChild::RequestThisPeer,
                Box::new(|mut ud, response| {
                    let response = {
                        match response {
                            GhostCallbackData::Timeout => panic!("timeout"),
                            GhostCallbackData::Response(response) => match response {
                                Err(e) => panic!("{:?}", e),
                                Ok(response) => response,
                            },
                        }
                    };
                    if let DhtRequestToChildResponse::RequestThisPeer(peer_response) = response {
                        ud.this_peer = peer_response;
                    } else {
                        panic!("bad response to bind: {:?}", response);
                    }
                    Ok(())
                }),
            )
            .expect("sync functions should work");
        self.inner_dht.process(&mut self.user_data).unwrap(); // FIXME unwrap
        self.this_peer = self.user_data.this_peer.clone();
        self.this_peer.clone()
    }

    ///
    pub fn get_peer_sync(&mut self, peer_address: &str) -> Option<PeerData> {
        self.inner_dht
            .request(
                Lib3hSpan::todo(),
                DhtRequestToChild::RequestPeer(peer_address.to_string()),
                Box::new(|mut ud, response| {
                    let response = {
                        match response {
                            GhostCallbackData::Timeout => panic!("timeout"),
                            GhostCallbackData::Response(response) => match response {
                                Err(e) => panic!("{:?}", e),
                                Ok(response) => response,
                            },
                        }
                    };
                    if let DhtRequestToChildResponse::RequestPeer(peer_response) = response {
                        ud.maybe_peer = peer_response;
                    } else {
                        panic!("bad response to bind: {:?}", response);
                    }
                    Ok(())
                }),
            )
            .expect("sync functions should work");
        let _res = self.inner_dht.process(&mut self.user_data).unwrap(); // FIXME unwrap
        self.user_data.maybe_peer.clone()
    }
}
