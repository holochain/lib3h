#![allow(non_snake_case)]

use crate::{
    dht::{dht_trait::DhtConfig, ghost_protocol::*, dht_protocol::*},

    gateway::{Gateway, P2pGateway},
    transport::{protocol::*, TransportWrapper},
};
use lib3h_protocol::Address;
use std::collections::{HashMap, VecDeque};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use url::Url;
use std::any::Any;

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
            inner_dht: Detach::new(ChildDhtWrapperDyn::new(dht, "gateway_dht")),
            identifier: identifier.to_owned(),
            connection_map: HashMap::new(),
            transport_inbox: VecDeque::new(),
            transport_inject_events: Vec::new(),
            maybe_peer: None,
            this_peer: PeerData {
                peer_address: String::new(),
                peer_uri: Url::parse("dummy://default").unwrap(),
                timestamp: 0,
            },
            peer_list: Vec::new(),
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

    fn process_dht(&mut self, user_data: &mut dyn Any) -> GhostResult<()> {
        self.inner_dht.process(user_data)
    }

//    fn take_dht_parent_endpoint(&mut self) -> Option<DhtEndpoint> {
//        self.inner_dht.take_parent_endpoint()
//    }

    fn as_dht_mut(&mut self) -> &mut Detach<ChildDhtWrapperDyn> {
        &mut self.inner_dht
    }

    ///
    fn get_peer_list_sync(&mut self) -> Vec<PeerData> {
        self.inner_dht.request(
            std::time::Duration::from_millis(2000),
            DhtContext::NoOp,
            DhtRequestToChild::RequestPeerList,
            Box::new(|me, _context, response| {
                let mut gateway = match me.downcast_mut::<P2pGateway>() {
                    None => panic!("bad downcast"),
                    Some(e) => e,
                };
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => match response {
                            Err(e) => panic!("{:?}", e),
                            Ok(response) => response,
                        },
                    }
                };
                if let DhtRequestToChildResponse::RequestPeerList(peer_list_response) = response {
                    gateway.peer_list = peer_list_response;
                } else {
                    panic!("bad response to bind: {:?}", response);
                }
                Ok(())
            }),
        );
        let _res = self.inner_dht.process(&mut ()).unwrap(); // FIXME unwrap
//        //let _ = parent_endpoint.process(&mut ());
//        // Wait for bind response
//        let mut timeout = 0;
//        while peer_list == Vec::new() && timeout < 200 {
//            std::thread::sleep(std::time::Duration::from_millis(10));
//            timeout += 10;
//        }
//        assert!(timeout < 200);
        self.peer_list.clone()
    }

    ///
    fn get_this_peer_sync(&mut self) -> PeerData {
        // get cached value first
        if self.this_peer.peer_address != String::new() {
            return self.this_peer.clone();
        }
        self.inner_dht.request(
            std::time::Duration::from_millis(2000),
            DhtContext::NoOp,
            DhtRequestToChild::RequestThisPeer,
            Box::new(|me, _context, response| {
                let mut gateway = match me.downcast_mut::<P2pGateway>() {
                    None => panic!("bad downcast"),
                    Some(e) => e,
                };
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
                    gateway.this_peer = peer_response;
                } else {
                    panic!("bad response to bind: {:?}", response);
                }
                Ok(())
            }),
        );
        let _res = self.inner_dht.process(&mut ()).unwrap(); // FIXME unwrap
//        //let _ = parent_endpoint.process(&mut ());
//        // Wait for bind response
//        let mut timeout = 0;
//        while this_peer.peer_address == String::new() && timeout < 200 {
//            std::thread::sleep(std::time::Duration::from_millis(10));
//            timeout += 10;
//        }
//        assert!(timeout < 200);
        self.this_peer.clone()
    }

    ///
    fn get_peer_sync(&mut self, peer_address: &str) -> Option<PeerData> {
        self.inner_dht.request(
            std::time::Duration::from_millis(2000),
            DhtContext::NoOp,
            DhtRequestToChild::RequestPeer(peer_address.to_string()),
            Box::new(|me, _context, response| {
                let mut gateway = match me.downcast_mut::<P2pGateway>() {
                    None => panic!("bad downcast"),
                    Some(e) => e,
                };
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
                    gateway.maybe_peer = peer_response;
                } else {
                    panic!("bad response to bind: {:?}", response);
                }
                Ok(())
            }),
        );
        let _res = self.inner_dht.process(&mut ()).unwrap(); // FIXME unwrap
//        //let _ = parent_endpoint.process(&mut ());
//        // Wait for bind response
//        let mut timeout = 0;
//        while maybe_peer == None && timeout < 200 {
//            std::thread::sleep(std::time::Duration::from_millis(10));
//            timeout += 10;
//        }
        self.maybe_peer.clone()
    }

//    fn as_dht_mut(&mut self) -> &mut Detach<ChildDhtWrapperDyn> {
//        &mut self.inner_dht
//    }
}

