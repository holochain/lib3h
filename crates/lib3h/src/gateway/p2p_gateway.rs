#![allow(non_snake_case)]

use crate::{
    dht::{dht_config::DhtConfig, dht_protocol::*},
    engine::NETWORK_GATEWAY_ID,
    gateway::{Gateway, GatewayUserData, P2pGateway},
    transport::{protocol::*, TransportWrapper},
};
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{protocol_server::Lib3hServerProtocol, Address};
use lib3h_tracing::Lib3hSpan;
use std::collections::{HashMap, VecDeque};
use url::Url;

//--------------------------------------------------------------------------------------------------
// Constructors
//--------------------------------------------------------------------------------------------------

/// P2pGateway Constructors
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
            inner_dht: ChildDhtWrapperDyn::new(dht, "gateway_dht"),
            identifier: identifier.to_owned(),
            connection_map: HashMap::new(),
            transport_inbox: VecDeque::new(),
            transport_inject_events: Vec::new(),
            this_peer: PeerData {
                peer_address: String::new(),
                peer_uri: Url::parse("dummy://default").unwrap(),
                timestamp: 0,
            },
            user_data: GatewayUserData::new(),
        }
    }
    /// Helper Ctor
    pub fn new_with_space(
        space_address: &Address,
        network_gateway: TransportWrapper<'gateway>,
        dht_factory: DhtFactory,
        dht_config: &DhtConfig,
    ) -> Self {
        let identifier: String = space_address.clone().into();
        P2pGateway::new(&identifier, network_gateway, dht_factory, dht_config)
    }
}

/// Gateway Trait
impl<'gateway> Gateway for P2pGateway<'gateway> {
    /// This Gateway's identifier
    fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    fn transport_inject_event(&mut self, evt: TransportEvent) {
        self.transport_inject_events.push(evt);
    }

    /// Helper for getting a connectionId from a peer_address
    fn get_connection_id(&mut self, peer_address: &str) -> Option<String> {
        // get peer_uri
        let maybe_peer_data = self.get_peer_sync(peer_address);
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

    fn process_dht(&mut self) -> GhostResult<()> {
        let res = self.inner_dht.process(&mut self.user_data);
        res
    }

    fn as_dht_mut(&mut self) -> &mut ChildDhtWrapperDyn<GatewayUserData> {
        &mut self.inner_dht
    }

    fn drain_dht_outbox(&mut self) -> Vec<Lib3hServerProtocol> {
        self.user_data.lib3h_outbox.drain(0..).collect()
    }

    // TODO - remove this hack
    fn hold_peer(&mut self, peer_data: PeerData) {
        if self.identifier != NETWORK_GATEWAY_ID {
            debug!(
                "({}).Dht.post(HoldPeer) - {}",
                self.identifier, peer_data.peer_uri,
            );
            // In space_gateway `peer_uri` is a URI-ed transportId, so un-URI-ze it
            // to get the transportId
            let maybe_previous = self.connection_map.insert(
                peer_data.peer_uri.clone(),
                String::from(peer_data.peer_uri.path()),
            );
            if let Some(previous_cId) = maybe_previous {
                debug!(
                    "Replaced connectionId for {} ; was: {}",
                    peer_data.peer_uri.clone(),
                    previous_cId
                );
            }
        }
        let _ = self
            .inner_dht
            .publish(Lib3hSpan::todo(), DhtRequestToChild::HoldPeer(peer_data));
    }

    ///
    fn get_peer_list_sync(&mut self) -> Vec<PeerData> {
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
    fn get_this_peer_sync(&mut self) -> PeerData {
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
    fn get_peer_sync(&mut self, peer_address: &str) -> Option<PeerData> {
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
