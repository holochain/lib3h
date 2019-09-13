pub mod dht_config;
pub mod dht_protocol;
pub mod mirror_dht;
//pub mod rrdht;

/// a Peer identifier
pub type PeerAddress = String;
pub type PeerAddressRef = str;

#[cfg(test)]
pub mod tests {
    use crate::{
        dht::{dht_protocol::*, mirror_dht::MirrorDht, PeerAddressRef},
        tests::enable_logging_for_test,
    };
    use detach::prelude::*;
    use lib3h_ghost_actor::prelude::*;
    use lib3h_protocol::{
        data_types::{EntryAspectData, EntryData},
        Address,
    };
    use lib3h_tracing::{test_span, Lib3hSpan};
    use url::Url;

    lazy_static! {
        /// CONSTS
        /// Entries
        pub static ref ENTRY_ADDRESS_1: Address = "entry_addr_1".into();
        pub static ref ENTRY_ADDRESS_2: Address = "entry_addr_2".into();
        pub static ref ENTRY_ADDRESS_3: Address = "entry_addr_3".into();
        /// Aspects
        pub static ref ASPECT_CONTENT_1: Vec<u8> = "hello-1".as_bytes().to_vec();
        pub static ref ASPECT_CONTENT_2: Vec<u8> = "l-2".as_bytes().to_vec();
        pub static ref ASPECT_CONTENT_3: Vec<u8> = "ChainHeader-3".as_bytes().to_vec();
        pub static ref ASPECT_ADDRESS_1: Address = "aspect_addr_1".into();
        pub static ref ASPECT_ADDRESS_2: Address = "aspect_addr_2".into();
        pub static ref ASPECT_ADDRESS_3: Address = "aspect_addr_3".into();
    }

    const PEER_A: &PeerAddressRef = "alex";
    const PEER_B: &PeerAddressRef = "billy";
    const PEER_C: &PeerAddressRef = "camille";

    // Request counters
    #[allow(dead_code)]
    static mut FETCH_COUNT: u32 = 0;

    pub struct DhtData {
        this_peer: PeerData,
        maybe_peer: Option<PeerData>,
        peer_list: Vec<PeerData>,
        entry_list: Vec<Address>,
        maybe_aspect_list: Option<Vec<Address>>,
    }

    impl DhtData {
        pub fn new() -> Self {
            DhtData {
                this_peer: PeerData {
                    peer_address: "FIXME".to_string(),
                    peer_uri: Url::parse("fixme://host:123").unwrap(),
                    timestamp: 0,
                },
                maybe_peer: None,
                peer_list: Vec::new(),
                entry_list: Vec::new(),
                maybe_aspect_list: None,
            }
        }
    }

    fn create_test_uri(peer_address: &PeerAddressRef) -> Url {
        Url::parse(format!("test://{}", peer_address).as_str()).unwrap()
    }

    #[allow(non_snake_case)]
    fn create_PeerData(peer_address: &PeerAddressRef) -> PeerData {
        PeerData {
            peer_address: peer_address.to_owned(),
            peer_uri: create_test_uri(peer_address),
            timestamp: crate::time::since_epoch_ms(),
        }
    }

    #[allow(non_snake_case)]
    fn create_EntryData(
        entry_address: &Address,
        aspect_address: &Address,
        aspect_content: &[u8],
    ) -> EntryData {
        let aspect = EntryAspectData {
            aspect_address: aspect_address.to_owned(),
            type_hint: "dht_test".to_string(),
            aspect: aspect_content.into(),
            publish_ts: crate::time::since_epoch_ms(),
        };
        EntryData {
            entry_address: entry_address.to_owned(),
            aspect_list: vec![aspect],
        }
    }

    #[allow(non_snake_case)]
    #[allow(dead_code)]
    fn create_FetchEntry(entry_address: &Address) -> FetchDhtEntryData {
        unsafe {
            FETCH_COUNT += 1;
            FetchDhtEntryData {
                msg_id: format!("fetch_{}", FETCH_COUNT),
                entry_address: entry_address.to_owned(),
            }
        }
    }

    fn new_dht(_is_mirror: bool, peer_address: &PeerAddressRef) -> Box<DhtActor> {
        //if is_mirror {
        return MirrorDht::new(peer_address, &create_test_uri(peer_address));
        //}
        //Box::new(RrDht::new())
    }

    fn new_dht_wrapper(
        _is_mirror: bool,
        peer_address: &PeerAddressRef,
    ) -> Detach<ChildDhtWrapperDyn<DhtData>> {
        let dht = new_dht(true, peer_address);
        Detach::new(ChildDhtWrapperDyn::new(dht, "dht_parent_"))
    }

    fn get_this_peer(dht: &mut Detach<ChildDhtWrapperDyn<DhtData>>) -> PeerData {
        let mut ud = DhtData::new();
        dht.request(
            test_span(""),
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
                    panic!("bad response to RequestThisPeer: {:?}", response);
                }
                Ok(())
            }),
        )
        .unwrap();
        println!("dht.process(get_this_peer)...");
        dht.process(&mut ud).unwrap();
        ud.this_peer
    }

    fn get_peer(dht: &mut Detach<ChildDhtWrapperDyn<DhtData>>, address: &str) -> Option<PeerData> {
        let mut ud = DhtData::new();
        dht.request(
            test_span(""),
            DhtRequestToChild::RequestPeer(address.to_string()),
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
                    panic!("bad response to RequestPeer: {:?}", response);
                }
                Ok(())
            }),
        )
        .unwrap();
        println!("dht.process(get_peer) ...");
        dht.process(&mut ud).unwrap();
        ud.maybe_peer
    }

    fn get_peer_list(dht: &mut Detach<ChildDhtWrapperDyn<DhtData>>) -> Vec<PeerData> {
        let mut ud = DhtData::new();
        dht.request(
            test_span(""),
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
                if let DhtRequestToChildResponse::RequestPeerList(peer_response) = response {
                    ud.peer_list = peer_response;
                } else {
                    panic!("bad response to RequestPeerList: {:?}", response);
                }
                Ok(())
            }),
        )
        .unwrap();
        println!("dht.process(get_peer_list)...");
        dht.process(&mut ud).unwrap();
        ud.peer_list
    }

    fn get_entry_address_list(dht: &mut Detach<ChildDhtWrapperDyn<DhtData>>) -> Vec<Address> {
        let mut ud = DhtData::new();
        dht.request(
            test_span(""),
            DhtRequestToChild::RequestEntryAddressList,
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
                if let DhtRequestToChildResponse::RequestEntryAddressList(entry_response) = response
                {
                    ud.entry_list = entry_response;
                } else {
                    panic!("bad response to RequestEntryAddressList: {:?}", response);
                }
                Ok(())
            }),
        )
        .unwrap();
        println!("dht.process(get_entry_address_list)...");
        dht.process(&mut ud).unwrap();
        ud.entry_list
    }

    fn get_aspects_of(
        dht: &mut Detach<ChildDhtWrapperDyn<DhtData>>,
        entry_address: &Address,
    ) -> Option<Vec<Address>> {
        let mut ud = DhtData::new();
        dht.request(
            test_span(""),
            DhtRequestToChild::RequestAspectsOf(entry_address.clone()),
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
                if let DhtRequestToChildResponse::RequestAspectsOf(entry_response) = response {
                    ud.maybe_aspect_list = entry_response;
                } else {
                    panic!("bad response to RequestAspectsOf: {:?}", response);
                }
                Ok(())
            }),
        )
        .unwrap();
        println!("dht.process(get_aspects_of)...");
        dht.process(&mut ud).unwrap();
        ud.maybe_aspect_list
    }

    #[test]
    fn test_this_peer() {
        enable_logging_for_test(true);
        let mut dht = new_dht_wrapper(true, PEER_A);
        let this_peer = get_this_peer(&mut dht);
        assert_eq!(this_peer.peer_address, PEER_A);
    }

    #[test]
    fn test_own_peer_list() {
        enable_logging_for_test(true);
        let mut dht = new_dht_wrapper(true, PEER_A);
        let mut ud = DhtData::new();
        // Should be empty
        let maybe_peer = get_peer(&mut dht, PEER_A);
        assert!(maybe_peer.is_none());
        let peer_list = get_peer_list(&mut dht);
        assert_eq!(peer_list.len(), 0);
        // Add a peer
        dht.publish(
            Lib3hSpan::todo(),
            DhtRequestToChild::HoldPeer(create_PeerData(PEER_B)),
        )
        .unwrap();
        dht.process(&mut ud).unwrap();
        // Should have it
        let peer = get_peer(&mut dht, PEER_B).unwrap();
        assert_eq!(peer.peer_address, PEER_B);
        let peer_list = get_peer_list(&mut dht);
        assert_eq!(peer_list.len(), 1);
        assert_eq!(peer_list[0].peer_address, PEER_B);
        // Add a peer again
        dht.publish(
            Lib3hSpan::todo(),
            DhtRequestToChild::HoldPeer(create_PeerData(PEER_C)),
        )
        .unwrap();
        dht.process(&mut ud).unwrap();
        // Should have it
        let peer = get_peer(&mut dht, PEER_B).unwrap();
        assert_eq!(peer.peer_address, PEER_B);
        let peer_list = get_peer_list(&mut dht);
        assert_eq!(peer_list.len(), 2);
    }

    #[test]
    fn test_get_own_entry() {
        enable_logging_for_test(true);
        let mut dht = new_dht_wrapper(true, PEER_A);
        let mut ud = DhtData::new();
        // Should be empty
        let entry_address_list = get_entry_address_list(&mut dht);
        assert_eq!(entry_address_list.len(), 0);
        // Add a data item
        let entry = create_EntryData(&ENTRY_ADDRESS_1, &ASPECT_ADDRESS_1, &ASPECT_CONTENT_1);
        println!("dht.process(HoldEntryAspectAddress)...");
        dht.publish(
            Lib3hSpan::todo(),
            DhtRequestToChild::HoldEntryAspectAddress(entry.clone()),
        )
        .unwrap();
        dht.process(&mut ud).unwrap();
        // Should have it
        let entry_address_list = get_entry_address_list(&mut dht);
        assert_eq!(entry_address_list.len(), 1);
        let maybe_aspects = get_aspects_of(&mut dht, &*ENTRY_ADDRESS_1);
        assert!(maybe_aspects.is_some());
        assert_eq!(maybe_aspects.unwrap().len(), 1);
        // Flush any pending requests from child
        let request_list = dht.drain_messages();
        println!("0. dht.drain_messages(): {}", request_list.len());
        // Fetch it
        // ========
        dht.request(
            test_span(""),
            DhtRequestToChild::RequestEntry(ENTRY_ADDRESS_1.clone()),
            Box::new(|_ud, response| {
                println!("5. In DhtRequestToChild::RequestEntry Response Closure");
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => match response {
                            Err(e) => panic!("{:?}", e),
                            Ok(response) => response,
                        },
                    }
                };
                if let DhtRequestToChildResponse::RequestEntry(entry_response) = response {
                    // Should have it
                    assert_eq!(entry_response.entry_address, *ENTRY_ADDRESS_1);
                } else {
                    panic!("bad response to RequestAspectsOf: {:?}", response);
                }
                Ok(())
            }),
        )
        .unwrap();
        println!("1. dht.process(RequestEntry)...");
        dht.process(&mut ud).unwrap();
        // Should have received the request back
        let request_list = dht.drain_messages();
        assert_eq!(request_list.len(), 1);
        for mut request in request_list {
            println!("2. request = {:?}", request);
            match request.take_message().expect("exists") {
                DhtRequestToParent::RequestEntry(entry_address) => {
                    assert_eq!(entry_address, *ENTRY_ADDRESS_1);
                }
                _ => panic!("Expecting a different request type"),
            }
            request
                .respond(Ok(DhtRequestToParentResponse::RequestEntry(entry.clone())))
                .unwrap();
        }
        println!("3. dht.process(RequestEntry)...");
        dht.process(&mut ud).unwrap();
    }

    #[test]
    fn test_update_peer() {
        enable_logging_for_test(true);
        let mut dht = new_dht_wrapper(true, PEER_A);
        let mut ud = DhtData::new();
        // Should be empty
        let this = get_peer(&mut dht, PEER_A);
        assert!(this.is_none());
        let peer_list = get_peer_list(&mut dht);
        assert_eq!(peer_list.len(), 0);
        // Add a peer
        // wait a bit so that the -1 does not underflow
        // TODO #211
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut peer_b_data = create_PeerData(PEER_B);
        dht.publish(
            Lib3hSpan::todo(),
            DhtRequestToChild::HoldPeer(peer_b_data.clone()),
        )
        .unwrap();
        dht.process(&mut ud).unwrap();
        // Should have it
        let peer = get_peer(&mut dht, PEER_B).unwrap();
        assert_eq!(peer.peer_address, PEER_B);
        // Add older peer info
        let ref_time = peer_b_data.timestamp;
        peer_b_data.timestamp -= 1;
        dht.publish(
            Lib3hSpan::todo(),
            DhtRequestToChild::HoldPeer(peer_b_data.clone()),
        )
        .unwrap();
        dht.process(&mut ud).unwrap();
        // Should have unchanged timestamp
        let peer = get_peer(&mut dht, PEER_B).unwrap();
        assert_eq!(peer.timestamp, ref_time);
        // Add newer peer info
        // wait a bit so that the +1 is not ahead of 'now'
        // TODO #211
        std::thread::sleep(std::time::Duration::from_millis(10));
        peer_b_data.timestamp = ref_time + 1;
        dht.publish(Lib3hSpan::todo(), DhtRequestToChild::HoldPeer(peer_b_data))
            .unwrap();
        dht.process(&mut ud).unwrap();
        // Should have unchanged timestamp
        let peer = get_peer(&mut dht, PEER_B).unwrap();
        assert!(peer.timestamp > ref_time);
    }

    #[test]
    fn test_mirror_broadcast_entry() {
        enable_logging_for_test(true);
        let mut dht_a = new_dht_wrapper(true, PEER_A);
        let mut dht_b = new_dht_wrapper(true, PEER_B);
        let mut ud = DhtData::new();
        // Add a peer
        dht_a
            .publish(
                Lib3hSpan::todo(),
                DhtRequestToChild::HoldPeer(create_PeerData(PEER_B)),
            )
            .unwrap();
        dht_a.process(&mut ud).unwrap();
        // Flush any pending requests from child
        let request_list = dht_a.drain_messages();
        println!("dht_a.drain_messages(): {}", request_list.len());
        // Add a data item in DHT A
        let entry_data = create_EntryData(&ENTRY_ADDRESS_1, &ASPECT_ADDRESS_1, &ASPECT_CONTENT_1);
        dht_a
            .publish(
                Lib3hSpan::todo(),
                DhtRequestToChild::BroadcastEntry(entry_data.clone()),
            )
            .unwrap();
        dht_a.process(&mut ud).unwrap();
        // Should return a gossipTo
        let request_list = dht_a.drain_messages();
        assert_eq!(request_list.len(), 1);
        let mut bundle: lib3h_protocol::data_types::Opaque = "".into();
        for mut request in request_list {
            match request.take_message().expect("exists") {
                DhtRequestToParent::GossipTo(gossip_data) => {
                    assert_eq!(gossip_data.peer_address_list.len(), 1);
                    assert_eq!(gossip_data.peer_address_list[0], PEER_B);
                    bundle = gossip_data.bundle.clone();
                }
                _ => panic!("Expecting a different request type"),
            }
        }

        // Flush any pending requests from child
        let request_list = dht_b.drain_messages();
        println!("dht_b.drain_messages(): {}", request_list.len());
        // Post a remoteGossipTo
        let remote_gossip = RemoteGossipBundleData {
            from_peer_address: PEER_A.to_owned(),
            bundle,
        };
        dht_b
            .publish(
                Lib3hSpan::todo(),
                DhtRequestToChild::HandleGossip(remote_gossip),
            )
            .unwrap();
        dht_b.process(&mut ud).unwrap();
        // Should receive a HoldRequested
        let request_list = dht_b.drain_messages();
        assert_eq!(request_list.len(), 1);
        for mut request in request_list {
            match request.take_message().expect("exists") {
                DhtRequestToParent::HoldEntryRequested { from_peer, entry } => {
                    assert_eq!(from_peer, PEER_B.clone());
                    assert_eq!(entry, entry_data.clone());
                }
                _ => panic!("Expecting a different request type"),
            }
        }
        // Tell DHT B to hold it
        dht_b
            .publish(
                Lib3hSpan::todo(),
                DhtRequestToChild::HoldEntryAspectAddress(entry_data),
            )
            .unwrap();
        dht_b.process(&mut ud).unwrap();
        // DHT B should have the entry
        let entry_list = get_entry_address_list(&mut dht_b);
        assert_eq!(entry_list.len(), 1);
    }

    #[test]
    fn test_mirror_gossip_peer() {
        enable_logging_for_test(true);
        let mut dht_a = new_dht_wrapper(true, PEER_A);
        let mut dht_b = new_dht_wrapper(true, PEER_B);
        let mut ud = DhtData::new();
        // Tell A to hold B
        let peer_b_data = get_this_peer(&mut dht_b);
        assert_eq!(peer_b_data.peer_address, PEER_B);
        dht_a
            .publish(
                Lib3hSpan::todo(),
                DhtRequestToChild::HoldPeer(peer_b_data.clone()),
            )
            .unwrap();
        dht_a.process(&mut ud).unwrap();
        // Flush any pending requests from child
        let request_list = dht_a.drain_messages();
        println!("dht_a.drain_messages(): {}", request_list.len());
        // Tell A to hold C
        let peer_c_data = create_PeerData(PEER_C);
        dht_a
            .publish(
                Lib3hSpan::todo(),
                DhtRequestToChild::HoldPeer(peer_c_data.clone()),
            )
            .unwrap();
        dht_a.process(&mut ud).unwrap();
        // Should return gossipTos of C to B
        let request_list = dht_a.drain_messages();
        assert_eq!(request_list.len(), 2);
        let mut bundle: lib3h_protocol::data_types::Opaque = "".into();
        for mut request in request_list {
            match request.take_message().expect("exists") {
                DhtRequestToParent::GossipTo(gossip_to) => {
                    println!("gossip_to = {:?}", gossip_to);
                    assert_eq!(gossip_to.peer_address_list.len(), 1);
                    assert!(
                        gossip_to.peer_address_list[0] == PEER_C
                            || gossip_to.peer_address_list[0] == PEER_B
                    );
                    if gossip_to.peer_address_list[0] == PEER_B {
                        bundle = gossip_to.bundle.clone();
                    }
                }
                _ => panic!("Expecting a different request type"),
            }
        }
        // Flush any pending requests from child
        let request_list = dht_b.drain_messages();
        println!("dht_b.drain_messages(): {}", request_list.len());
        // Tell B to hold C from A's gossip
        let remote_gossip = RemoteGossipBundleData {
            from_peer_address: PEER_A.to_owned(),
            bundle,
        };
        dht_b
            .publish(
                Lib3hSpan::todo(),
                DhtRequestToChild::HandleGossip(remote_gossip),
            )
            .unwrap();
        dht_b.process(&mut ud).unwrap();
        // Should return gossipTos
        let request_list = dht_b.drain_messages();
        assert_eq!(request_list.len(), 1);
        let mut peer_to_hold = PeerData {
            peer_address: "FIXME".to_string(),
            peer_uri: Url::parse("fixme://host:123").unwrap(),
            timestamp: 0,
        };
        for mut request in request_list {
            match request.take_message().expect("exists") {
                DhtRequestToParent::HoldPeerRequested(peer) => {
                    peer_to_hold = peer;
                    println!("peer_to_hold = {:?}", peer_to_hold);
                }
                _ => panic!("Expecting a different request type"),
            }
        }
        // Accept HoldPeerRequested
        dht_b
            .publish(
                Lib3hSpan::todo(),
                DhtRequestToChild::HoldPeer(peer_to_hold.clone()),
            )
            .unwrap();
        dht_b.process(&mut ud).unwrap();
        // B should have C
        let peer_info = get_peer(&mut dht_b, PEER_C).unwrap();
        assert_eq!(peer_info, peer_c_data);
    }
}
