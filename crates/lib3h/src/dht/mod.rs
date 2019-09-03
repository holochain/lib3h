pub mod dht_protocol;
pub mod dht_trait;
pub mod mirror_dht;
//pub mod rrdht;
pub mod ghost_protocol;

/// a Peer identifier
pub type PeerAddress = String;
pub type PeerAddressRef = str;

#[cfg(test)]
pub mod tests {
    use crate::{
        dht::{
            ghost_protocol::*,
            dht_protocol::*,
            mirror_dht::MirrorDht,
            // rrdht::RrDht,
            PeerAddressRef,
        },
        tests::enable_logging_for_test,
    };
    use lib3h_protocol::{
        data_types::{EntryAspectData, EntryData},
        Address,
    };
    use url::Url;
    use detach::prelude::*;
    use lib3h_ghost_actor::prelude::*;

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

//    pub struct DhtParent {
//        inner_dht: Detach<ChildDhtWrapperDyn>,
//        /// temp variables for ghostCallback mutation
//        maybe_peer: Option<PeerData>,
//        this_peer: PeerData,
//        peer_list: Vec<PeerData>,
//    }
//    impl DhtParent {
//        pub fn new(dht: Box<DhtActor>) -> Self {
//            DhtParent {
//                inner_dht: Detach::new(ChildDhtWrapperDyn::new(dht, "dht_parent")),
//                maybe_peer: None,
//                this_peer: PeerData {
//                    peer_address: String::new(),
//                    peer_uri: Url::parse("dummy://default").unwrap(),
//                    timestamp: 0,
//                },
//                peer_list: Vec::new(),
//            }
//        }
//    }

    // Request counters
    #[allow(dead_code)]
    static mut FETCH_COUNT: u32 = 0;

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
            aspect: aspect_content.to_owned(),
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
//
//    fn new_dht_parent(is_mirror: bool, peer_address: &PeerAddressRef) -> DhtParent {
//        DhtParent::new(new_dht(is_mirror, peer_address))
//    }

    fn new_dht_wrapper(_is_mirror: bool, peer_address: &PeerAddressRef) -> Detach<ChildDhtWrapperDyn> {
        let dht = new_dht(true, PEER_A);
        Detach::new(ChildDhtWrapperDyn::new(dht, "dht_parent"))
    }

    #[test]
    fn test_this_peer() {
        enable_logging_for_test(true);
        let mut dht = new_dht_wrapper(true, PEER_A);
        let mut this_peer = PeerData {
            peer_address: String::new(),
            peer_uri: Url::parse("dummy://default").unwrap(),
            timestamp: 0,
        };
        dht.request(
            std::time::Duration::from_millis(200),
            DhtContext::NoOp,
            DhtRequestToChild::RequestThisPeer,
            Box::new(|ud, _context, response| {
                let mut this_peer = match ud.downcast_mut::<PeerData>() {
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
                    *this_peer = peer_response;
                } else {
                    panic!("bad response to bind: {:?}", response);
                }
                Ok(())
            }),
        );
        println!("dht.process()...");
        dht.process(&mut this_peer).unwrap();
        assert_eq!(this_peer.peer_address, PEER_A);
    }

    #[test]
    fn test_own_peer_list() {
        enable_logging_for_test(true);
        let mut dht = new_dht_wrapper(true, PEER_A);
        // Should be empty
        let this = dht.get_peer(PEER_A);
        assert!(this.is_none());
        let peer_list = dht.get_peer_list();
        assert_eq!(peer_list.len(), 0);
        // Add a peer
        dht.post(DhtCommand::HoldPeer(create_PeerData(PEER_B)))
            .unwrap();
        let (did_work, _) = dht.process().unwrap();
        assert!(did_work);
        // Should have it
        let peer = dht.get_peer(PEER_B).unwrap();
        assert_eq!(peer.peer_address, PEER_B);
        let peer_list = dht.get_peer_list();
        assert_eq!(peer_list.len(), 1);
        assert_eq!(peer_list[0].peer_address, PEER_B);
        // Add a peer again
        dht.post(DhtCommand::HoldPeer(create_PeerData(PEER_C)))
            .unwrap();
        let (did_work, _) = dht.process().unwrap();
        assert!(did_work);
        // Should have it
        let peer = dht.get_peer(PEER_B).unwrap();
        assert_eq!(peer.peer_address, PEER_B);
        let peer_list = dht.get_peer_list();
        assert_eq!(peer_list.len(), 2);
    }
//
//    #[test]
//    fn test_get_own_entry() {
//        enable_logging_for_test(true);
//        let mut dht = new_dht(true, PEER_A);
//        // Should be empty
//        let entry_address_list = dht.get_entry_address_list();
//        assert_eq!(entry_address_list.len(), 0);
//        // Add a data item
//        let entry = create_EntryData(&ENTRY_ADDRESS_1, &ASPECT_ADDRESS_1, &ASPECT_CONTENT_1);
//        dht.post(DhtCommand::HoldEntryAspectAddress(entry.clone()))
//            .unwrap();
//        let (did_work, _) = dht.process().unwrap();
//        assert!(did_work);
//        // Should have it
//        let entry_address_list = dht.get_entry_address_list();
//        assert_eq!(entry_address_list.len(), 1);
//        let maybe_aspects = dht.get_aspects_of(&ENTRY_ADDRESS_1);
//        assert!(maybe_aspects.is_some());
//        assert_eq!(maybe_aspects.unwrap().len(), 1);
//        // Fetch it
//        let fetch_entry = FetchDhtEntryData {
//            msg_id: "fetch_1".to_owned(),
//            entry_address: ENTRY_ADDRESS_1.clone(),
//        };
//        dht.post(DhtCommand::FetchEntry(fetch_entry)).unwrap();
//        let (_did_work, event_list) = dht.process().unwrap();
//        assert_eq!(event_list.len(), 1);
//        let provide_entry = unwrap_to!(event_list[0] => DhtEvent::EntryDataRequested);
//        // Make something up
//        let response = FetchDhtEntryResponseData {
//            msg_id: provide_entry.msg_id.clone(),
//            entry: entry.clone(),
//        };
//        dht.post(DhtCommand::EntryDataResponse(response)).unwrap();
//        let (_did_work, event_list) = dht.process().unwrap();
//        // Should have it
//        assert_eq!(event_list.len(), 1);
//        let entry_response = unwrap_to!(event_list[0] => DhtEvent::FetchEntryResponse);
//        assert_eq!(entry_response.entry, entry);
//    }
//
//    #[test]
//    fn test_update_peer() {
//        enable_logging_for_test(true);
//        let mut dht = new_dht(true, PEER_A);
//        // Should be empty
//        let this = dht.get_peer(PEER_A);
//        assert!(this.is_none());
//        let peer_list = dht.get_peer_list();
//        assert_eq!(peer_list.len(), 0);
//        // Add a peer
//        // wait a bit so that the -1 does not underflow
//        // TODO #211
//        std::thread::sleep(std::time::Duration::from_millis(10));
//        let mut peer_b_data = create_PeerData(PEER_B);
//        dht.post(DhtCommand::HoldPeer(peer_b_data.clone())).unwrap();
//        let (did_work, _) = dht.process().unwrap();
//        assert!(did_work);
//        // Should have it
//        let peer = dht.get_peer(PEER_B).unwrap();
//        assert_eq!(peer.peer_address, PEER_B);
//        // Add older peer info
//        let ref_time = peer_b_data.timestamp;
//        peer_b_data.timestamp -= 1;
//        dht.post(DhtCommand::HoldPeer(peer_b_data.clone())).unwrap();
//        let (did_work, _) = dht.process().unwrap();
//        assert!(did_work);
//        // Should have unchanged timestamp
//        let peer = dht.get_peer(PEER_B).unwrap();
//        assert_eq!(peer.timestamp, ref_time);
//        // Add newer peer info
//        // wait a bit so that the +1 is not ahead of 'now'
//        // TODO #211
//        std::thread::sleep(std::time::Duration::from_millis(10));
//        peer_b_data.timestamp = ref_time + 1;
//        dht.post(DhtCommand::HoldPeer(peer_b_data)).unwrap();
//        let (did_work, _) = dht.process().unwrap();
//        assert!(did_work);
//        // Should have unchanged timestamp
//        let peer = dht.get_peer(PEER_B).unwrap();
//        assert!(peer.timestamp > ref_time);
//    }
//
//    #[test]
//    fn test_mirror_broadcast_entry() {
//        enable_logging_for_test(true);
//        let mut dht_a = new_dht(true, PEER_A);
//        let mut dht_b = new_dht(true, PEER_B);
//        // Add a peer
//        dht_a
//            .post(DhtCommand::HoldPeer(create_PeerData(PEER_B)))
//            .unwrap();
//        let (did_work, _) = dht_a.process().unwrap();
//        assert!(did_work);
//        // Add a data item in DHT A
//        let entry_data = create_EntryData(&ENTRY_ADDRESS_1, &ASPECT_ADDRESS_1, &ASPECT_CONTENT_1);
//        dht_a
//            .post(DhtCommand::BroadcastEntry(entry_data.clone()))
//            .unwrap();
//        let (did_work, gossip_list) = dht_a.process().unwrap();
//        assert!(did_work);
//        // Should return a gossipTo
//        assert_eq!(gossip_list.len(), 1);
//        let gossip_to = unwrap_to!(gossip_list[0] => DhtEvent::GossipTo);
//        assert_eq!(gossip_to.peer_address_list.len(), 1);
//        assert_eq!(gossip_to.peer_address_list[0], PEER_B);
//        // Post it as a remoteGossipTo
//        let remote_gossip = RemoteGossipBundleData {
//            from_peer_address: PEER_A.to_owned(),
//            bundle: gossip_to.bundle.clone(),
//        };
//        dht_b.post(DhtCommand::HandleGossip(remote_gossip)).unwrap();
//        let (did_work, event_list) = dht_b.process().unwrap();
//        assert!(did_work);
//        // Should receive a HoldRequested
//        assert_eq!(event_list.len(), 1);
//        if let DhtEvent::HoldEntryRequested {from_peer, entry} = event_list[0].clone() {
//            assert_eq!(from_peer, PEER_B.clone());
//            assert_eq!(entry, entry_data.clone());
//        } else {
//            panic!("Should be of variant type HoldEntryRequested");
//        }
//        // Tell DHT B to hold it
//        dht_b
//            .post(DhtCommand::HoldEntryAspectAddress(entry_data))
//            .unwrap();
//        let (did_work, _) = dht_b.process().unwrap();
//        assert!(did_work);
//        // DHT B should have the entry
//        let entry_list = dht_b.get_entry_address_list();
//        assert_eq!(entry_list.len(), 1);
//    }
//
//    #[test]
//    fn test_mirror_gossip_peer() {
//        enable_logging_for_test(true);
//        let mut dht_a = new_dht(true, PEER_A);
//        let mut dht_b = new_dht(true, PEER_B);
//        // Add a peer
//        let peer_b_data = dht_b.this_peer();
//        dht_a
//            .post(DhtCommand::HoldPeer(peer_b_data.clone()))
//            .unwrap();
//        let (did_work, _) = dht_a.process().unwrap();
//        assert!(did_work);
//        // Add a second peer
//        let peer_c_data = create_PeerData(PEER_C);
//        dht_a
//            .post(DhtCommand::HoldPeer(peer_c_data.clone()))
//            .unwrap();
//        let (did_work, gossip_list) = dht_a.process().unwrap();
//        assert!(did_work);
//        // Should return gossipTos
//        println!("gossip_list: {:?}", gossip_list);
//        assert_eq!(gossip_list.len(), 2);
//        // 2nd gossip should be a response
//        let gossip_to = unwrap_to!(gossip_list[1] => DhtEvent::GossipTo);
//        assert_eq!(gossip_to.peer_address_list.len(), 1);
//        assert_eq!(gossip_to.peer_address_list[0], PEER_C);
//        // 1st gossip should be propagation
//        let gossip_to = unwrap_to!(gossip_list[0] => DhtEvent::GossipTo);
//        assert_eq!(gossip_to.peer_address_list.len(), 1);
//        assert_eq!(gossip_to.peer_address_list[0], PEER_B);
//        // Post it as a remoteGossipTo
//        let remote_gossip = RemoteGossipBundleData {
//            from_peer_address: PEER_A.to_owned(),
//            bundle: gossip_to.bundle.clone(),
//        };
//        dht_b.post(DhtCommand::HandleGossip(remote_gossip)).unwrap();
//        let (did_work, event_list) = dht_b.process().unwrap();
//        assert!(did_work);
//        println!("event_list: {:?}", event_list);
//        assert_eq!(event_list.len(), 1);
//        let peer_to_hold = unwrap_to!(event_list[0] => DhtEvent::HoldPeerRequested);
//        // Hold requested peer
//        dht_b
//            .post(DhtCommand::HoldPeer(peer_to_hold.clone()))
//            .unwrap();
//        let (did_work, _) = dht_b.process().unwrap();
//        assert!(did_work);
//        // DHT B should have the data
//        let peer_info = dht_b.get_peer(PEER_C).unwrap();
//        assert_eq!(peer_info, peer_c_data);
//    }
}
