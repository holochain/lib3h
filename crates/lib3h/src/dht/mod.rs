pub mod dht_protocol;
pub mod dht_trait;
pub mod mirror_dht;
pub mod rrdht;

#[cfg(test)]
pub mod tests {

    use crate::dht::{dht_protocol::*, dht_trait::Dht, mirror_dht::MirrorDht, rrdht::RrDht};
    use lib3h_protocol::{
        data_types::{EntryAspectData, EntryData},
        Address, AddressRef,
    };

    /// CONSTS
    lazy_static! {
        /// Entries
        pub static ref ENTRY_ADDRESS_1: Address = "entry_addr_1".as_bytes().to_vec();
        pub static ref ENTRY_ADDRESS_2: Address = "entry_addr_2".as_bytes().to_vec();
        pub static ref ENTRY_ADDRESS_3: Address = "entry_addr_3".as_bytes().to_vec();
        /// Aspects
        pub static ref ASPECT_CONTENT_1: Vec<u8> = "hello-1".as_bytes().to_vec();
        pub static ref ASPECT_CONTENT_2: Vec<u8> = "l-2".as_bytes().to_vec();
        pub static ref ASPECT_CONTENT_3: Vec<u8> = "ChainHeader-3".as_bytes().to_vec();
        pub static ref ASPECT_ADDRESS_1: Address = "aspect_addr_1".as_bytes().to_vec();
        pub static ref ASPECT_ADDRESS_2: Address = "aspect_addr_2".as_bytes().to_vec();
        pub static ref ASPECT_ADDRESS_3: Address = "aspect_addr_3".as_bytes().to_vec();

    }

    const PEER_A: &str = "alex";
    const PEER_B: &str = "billy";
    const PEER_C: &str = "camille";

    // Request counters
    static mut FETCH_COUNT: u32 = 0;

    fn create_test_transport(peer_address: &str) -> String {
        format!("test://{}", peer_address)
    }

    #[allow(non_snake_case)]
    fn create_PeerData(peer_address: &str) -> PeerData {
        PeerData {
            peer_address: peer_address.to_owned(),
            transport: create_test_transport(peer_address),
            timestamp: 421,
        }
    }

    #[allow(non_snake_case)]
    fn create_EntryData(
        entry_address: &AddressRef,
        aspect_address: &AddressRef,
        aspect_content: &[u8],
    ) -> EntryData {
        let aspect = EntryAspectData {
            aspect_address: aspect_address.to_owned(),
            type_hint: "dht_test".to_string(),
            aspect: aspect_content.to_owned(),
            publish_ts: 123,
        };
        EntryData {
            entry_address: entry_address.to_owned(),
            aspect_list: vec![aspect],
        }
    }

    #[allow(non_snake_case)]
    fn create_FetchEntry(entry_address: &AddressRef) -> FetchEntryData {
        unsafe {
            FETCH_COUNT += 1;
            FetchEntryData {
                msg_id: format!("fetch_{}", FETCH_COUNT),
                entry_address: entry_address.to_owned(),
            }
        }
    }

    fn new_dht(is_mirror: bool, peer_address: &str) -> Box<dyn Dht> {
        if is_mirror {
            return Box::new(MirrorDht::new(
                peer_address,
                &create_test_transport(peer_address),
            ));
        }
        Box::new(RrDht::new())
    }

    #[test]
    fn test_this_peer() {
        let dht = new_dht(true, PEER_A);
        let this = dht.this_peer().unwrap();
        assert_eq!(this, PEER_A);
    }

    #[test]
    fn test_own_peer_list() {
        let mut dht = new_dht(true, PEER_A);
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

    #[test]
    fn test_get_own_data() {
        let mut dht = new_dht(true, PEER_A);
        // Should be empty
        let result = dht.get_entry(&ENTRY_ADDRESS_1);
        assert!(result.is_none());
        // Add a data item
        let entry_data = create_EntryData(&ENTRY_ADDRESS_1, &ASPECT_ADDRESS_1, &ASPECT_CONTENT_1);
        dht.post(DhtCommand::HoldEntry(entry_data.clone())).unwrap();
        let (did_work, _) = dht.process().unwrap();
        assert!(did_work);
        // Should have it
        let entry = dht.get_entry(&ENTRY_ADDRESS_1).unwrap();
        assert_eq!(entry, entry_data);
    }

    #[test]
    fn test_update_peer() {
        let mut dht = new_dht(true, PEER_A);
        // Should be empty
        let this = dht.get_peer(PEER_A);
        assert!(this.is_none());
        let peer_list = dht.get_peer_list();
        assert_eq!(peer_list.len(), 0);
        // Add a peer
        let mut peer_b_data = create_PeerData(PEER_B);
        dht.post(DhtCommand::HoldPeer(peer_b_data.clone())).unwrap();
        let (did_work, _) = dht.process().unwrap();
        assert!(did_work);
        // Should have it
        let peer = dht.get_peer(PEER_B).unwrap();
        assert_eq!(peer.peer_address, PEER_B);
        // Add older peer info
        let ref_time = peer_b_data.timestamp;
        peer_b_data.timestamp -= 1;
        dht.post(DhtCommand::HoldPeer(peer_b_data.clone())).unwrap();
        let (did_work, _) = dht.process().unwrap();
        assert!(did_work);
        // Should have unchanged timestamp
        let peer = dht.get_peer(PEER_B).unwrap();
        assert_eq!(peer.timestamp, ref_time);
        // Add newer peer info
        peer_b_data.timestamp = ref_time + 1;
        dht.post(DhtCommand::HoldPeer(peer_b_data)).unwrap();
        let (did_work, _) = dht.process().unwrap();
        assert!(did_work);
        // Should have unchanged timestamp
        let peer = dht.get_peer(PEER_B).unwrap();
        assert!(peer.timestamp > ref_time);
    }

    #[test]
    fn test_mirror_gossip_data() {
        let mut dht_a = new_dht(true, PEER_A);
        let mut dht_b = new_dht(true, PEER_B);
        // Add a peer
        dht_a
            .post(DhtCommand::HoldPeer(create_PeerData(PEER_B)))
            .unwrap();
        let (did_work, _) = dht_a.process().unwrap();
        assert!(did_work);
        // Add a data item
        let entry_data = create_EntryData(&ENTRY_ADDRESS_1, &ASPECT_ADDRESS_1, &ASPECT_CONTENT_1);
        dht_a
            .post(DhtCommand::HoldEntry(entry_data.clone()))
            .unwrap();
        let (did_work, gossip_list) = dht_a.process().unwrap();
        assert!(did_work);
        // Should return a gossipTo
        assert_eq!(gossip_list.len(), 1);
        let gossip_to = unwrap_to!(gossip_list[0] => DhtEvent::GossipTo);
        assert_eq!(gossip_to.peer_address_list.len(), 1);
        assert_eq!(gossip_to.peer_address_list[0], PEER_B);
        // Post it as a remoteGossipTo
        let remote_gossip = RemoteGossipBundleData {
            from_peer_address: PEER_A.to_string(),
            bundle: gossip_to.bundle.clone(),
        };
        dht_b.post(DhtCommand::HandleGossip(remote_gossip)).unwrap();
        let (did_work, _) = dht_b.process().unwrap();
        assert!(did_work);
        // DHT B should have the data
        let entry = dht_b.get_entry(&ENTRY_ADDRESS_1).unwrap();
        assert_eq!(entry, entry_data.clone());
        // DHT B should have the data with a Fetch
        let fetch_data = create_FetchEntry(&ENTRY_ADDRESS_1);
        let _ = dht_b
            .post(DhtCommand::FetchEntry(fetch_data.clone()))
            .unwrap();
        let (did_work, events) = dht_b.process().unwrap();
        assert!(did_work);
        assert_eq!(events.len(), 1);
        let fetch_response = unwrap_to!(events[0] => DhtEvent::FetchEntryResponse);
        assert_eq!(fetch_response.msg_id, fetch_data.msg_id);
        assert_eq!(fetch_response.entry, entry_data.clone());
    }

    #[test]
    fn test_mirror_gossip_peer() {
        let mut dht_a = new_dht(true, PEER_A);
        let mut dht_b = new_dht(true, PEER_B);
        // Add a peer
        let peer_b_data = create_PeerData(PEER_B);
        dht_a.post(DhtCommand::HoldPeer(peer_b_data)).unwrap();
        let (did_work, _) = dht_a.process().unwrap();
        assert!(did_work);
        // Add a second peer
        let peer_c_data = create_PeerData(PEER_C);
        dht_a
            .post(DhtCommand::HoldPeer(peer_c_data.clone()))
            .unwrap();
        let (did_work, gossip_list) = dht_a.process().unwrap();
        assert!(did_work);
        // Should return a gossipTo
        println!("gossip_list: {:?}", gossip_list);
        assert_eq!(gossip_list.len(), 1);
        let gossip_to = unwrap_to!(gossip_list[0] => DhtEvent::GossipTo);
        assert_eq!(gossip_to.peer_address_list.len(), 1);
        assert_eq!(gossip_to.peer_address_list[0], PEER_B);
        // Post it as a remoteGossipTo
        let remote_gossip = RemoteGossipBundleData {
            from_peer_address: PEER_A.to_string(),
            bundle: gossip_to.bundle.clone(),
        };
        dht_b.post(DhtCommand::HandleGossip(remote_gossip)).unwrap();
        let (did_work, _) = dht_b.process().unwrap();
        assert!(did_work);
        // DHT B should have the data
        let peer_info = dht_b.get_peer(PEER_C).unwrap();
        assert_eq!(peer_info, peer_c_data);
    }
}
