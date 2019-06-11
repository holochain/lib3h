pub mod dht_event;
pub mod dht_trait;
pub mod rrdht;
pub mod mirror_dht;

#[cfg(test)]
pub mod tests {

    #[macro_use]
    extern crate unwrap_to;

    use lib3h_protocol::{
        Address, AddressRef,
        data_types::{EntryData, EntryAspectData},
    };
    use crate::dht::{
        dht_event::*,
        mirror_dht::MirrorDht,
        rrdht::RrDht,
    };

    /// CONSTS
    lazy_static! {
        /// Entries
        pub static ref ENTRY_ADDRESS_1: Address = vec!["entry_addr_1"];
        pub static ref ENTRY_ADDRESS_2: Address = vec!["entry_addr_2"];
        pub static ref ENTRY_ADDRESS_3: Address = vec!["entry_addr_3"];
        /// Aspects
        pub static ref ASPECT_CONTENT_1: Vec<u8> = "hello-1".as_bytes().to_vec();
        pub static ref ASPECT_CONTENT_2: Vec<u8> = "l-2".as_bytes().to_vec();
        pub static ref ASPECT_CONTENT_3: Vec<u8> = "ChainHeader-3".as_bytes().to_vec();
        pub static ref ASPECT_ADDRESS_1: Address = vec!["aspect_addr_1"];
        pub static ref ASPECT_ADDRESS_2: Address = vec!["aspect_addr_2"];
        pub static ref ASPECT_ADDRESS_3: Address = vec!["aspect_addr_3"];
    }

    const PEER_A: &str = "alex";
    const PEER_B: &str = "billy";
    const PEER_C: &str = "camille";

    fn create_test_transport(peer_address: &str) -> String {
        format!("test://{}", peer_address)
    }

    fn create_PeerHoldRequest(peer_address: &str) -> DhtEvent {
        let data = PeerHoldRequestData {
            peer_address,
            transport: create_test_transport(peer_address),
            timestamp: 4.2,
        };
        DhtEvent::PeerHoldRequest(data)
    }

    fn create_DataHoldRequest(entry_address: &AddressRef, aspect_address: &AddressRef, aspect_content: &[u8]) -> DhtEvent {
        let aspect = EntryAspectData {
            aspect_address,
            type_hint: "dht_test",
            aspect: aspect_content,
            publish_ts: 0.1,
        };
        let entry = EntryData {
            entry_address,
            aspect_list: vec![aspect],
        };
        DhtEvent::DataHoldRequest(EntryHoldRequestData { entry })
    }

    fn new_dht(is_mirror: bool, peer_address: &str) -> impl Dht {
        if is_mirror {
            MirrorDht::new(peer_address, create_test_transport(peer_address))
        } else {
            RrDht::new()
        }
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
        assert_eq!(peer_list[0].peer_address, PEER_A);
        // Add a peer
        dht.post(create_PeerHoldRequest(PEER_B)).unwrap();
        let (did_work, _) = dht.process().unwrap();
        assert!(did_work);
        // Should have it
        let peer = dht.get_peer(PEER_B).unwrap();
        assert!(peer.peer_address, PEER_B);
        let peer_list = dht.get_peer_list();
        assert_eq!(peer_list.len(), 1);
        assert_eq!(peer_list[1].peer_address, PEER_B);
        // Add a peer again
        dht.post(create_PeerHoldRequest(PEER_C)).unwrap();
        let (did_work, _) = dht.process().unwrap();
        assert!(did_work);
        // Should have it
        let peer = dht.get_peer(PEER_B).unwrap();
        assert!(peer.peer_address, PEER_B);
        let peer_list = dht.get_peer_list();
        assert_eq!(peer_list.len(), 2);
    }

    #[test]
    fn test_get_own_data() {
        let dht = new_dht(true, PEER_A);
        // Should be empty
        let result = dht.get_data(ENTRY_ADDRESS_1);
        assert!(result.is_err());
        // Add a data item
        let data_hold = create_DataHoldRequest(ENTRY_ADDRESS_1, ASPECT_ADDRESS_1, ASPECT_CONTENT_1);
        dht.post(data_hold).unwrap();
        let (did_work, _) = dht.process().unwrap();
        assert!(did_work);
        // Should have it
        let entry = dht.get_data(ENTRY_ADDRESS_1).unwrap();
        assert_eq!(entry, data_hold.entry);
    }

    #[test]
    fn test_update_peer() {
        let mut dht = new_dht(true, PEER_A);
        // Should be empty
        let this = dht.get_peer(PEER_A);
        assert!(this.is_none());
        let peer_list = dht.get_peer_list();
        assert_eq!(peer_list.len(), 0);
        assert_eq!(peer_list[0].peer_address, PEER_A);
        // Add a peer
        let peer_hold = create_PeerHoldRequest(PEER_B);
        dht.post(peer_hold).unwrap();
        let (did_work, _) = dht.process().unwrap();
        assert!(did_work);
        // Should have it
        let peer = dht.get_peer(PEER_B).unwrap();
        assert!(peer.peer_address, PEER_B);
        // Add older peer info
        let mut peer_hold_data = unwrap_to!(peer_hold => DhtEvent::PeerHoldRequest);
        let ref_time = peer_hold_data.timestamp;
        peer_hold_data.timestamp -= 1;
        dht.post(DhtEvent::PeerHoldRequest(peer_hold_data)).unwrap();
        let (did_work, _) = dht.process().unwrap();
        assert!(did_work);
        // Should have unchanged timestamp
        let peer = dht.get_peer(PEER_B).unwrap();
        assert_eq!(peer.timestamp, ref_time);
        // Add newer peer info
        peer_hold_data.timestamp = ref_time + 1;
        dht.post(DhtEvent::PeerHoldRequest(peer_hold_data)).unwrap();
        let (did_work, _) = dht.process().unwrap();
        assert!(did_work);
        // Should have unchanged timestamp
        let peer = dht.get_peer(PEER_B).unwrap();
        assert!(peer.timestamp > ref_time);
    }

    #[test]
    fn test_mirror_gossip_data() {
        let dht_a = new_dht(true, PEER_A);
        let dht_b = new_dht(true, PEER_B);
        // Add a peer
        dht_a.post(create_PeerHoldRequest(PEER_B)).unwrap();
        let (did_work, _) = dht_a.process().unwrap();
        assert!(did_work);
        // Add a data item
        let data_hold = create_DataHoldRequest(ENTRY_ADDRESS_1, ASPECT_ADDRESS_1, ASPECT_CONTENT_1);
        dht_a.post(data_hold).unwrap();
        let (did_work, gossip_list) = dht_a.process().unwrap();
        assert!(did_work);
        // Should return a gossipTo
        assert_eq!(gossip_list.len(), 1);
        let gossip_to = unwrap_to!(gossip_list[0] => DhtEvent::GossipTo);
        assert_eq!(gossip_to.peer_address_list.len(), 1);
        assert_eq!(gossip_to.peer_address_list[0], PEER_A);
        // Post it as a remoteGossipTo
        let remote_gossip = RemoteGossipBundleData {
            from_peer_address: PEER_A.clone(),
            bundle: gossip_to.bundle,
        };
        dht_b.post(DhtEvent::RemoteGossipBundle(remote_gossip)).unwrap();
        let (did_work, _) = dht_b.process().unwrap();
        assert!(did_work);
        // DHT B should have the data
        let entry = dht_b.get_data(ENTRY_ADDRESS_1).unwrap();
        assert_eq!(entry, data_hold.entry);
    }

    #[test]
    fn test_mirror_gossip_peer() {
        let dht_a = new_dht(true, PEER_A);
        let dht_b = new_dht(true, PEER_B);
        // Add a peer
        dht_a.post(create_PeerHoldRequest(PEER_B)).unwrap();
        let (did_work, _) = dht_a.process().unwrap();
        assert!(did_work);
        // Add a second peer
        let peer_hold = create_PeerHoldRequest(PEER_C);
        dht_a.post(peer_hold).unwrap();
        let (did_work,gossip_list) = dht_a.process().unwrap();
        assert!(did_work);
        // Should return a gossipTo
        assert_eq!(gossip_list.len(), 1);
        let gossip_to = unwrap_to!(gossip_list[0] => DhtEvent::GossipTo);
        assert_eq!(gossip_to.peer_address_list.len(), 1);
        assert_eq!(gossip_to.peer_address_list[0], PEER_A);
        // Post it as a remoteGossipTo
        let remote_gossip = RemoteGossipBundleData {
            from_peer_address: PEER_A.clone(),
            bundle: gossip_to.bundle,
        };
        dht_b.post(DhtEvent::RemoteGossipBundle(remote_gossip)).unwrap();
        let (did_work, _) = dht_b.process().unwrap();
        assert!(did_work);
        // DHT B should have the data
        let peer_info = dht_b.get_peer(PEER_B).unwrap();
        let hold_info = unwrap_to!(peer_hold => DhtEvent::PeerHoldRequest);
        assert_eq!(peer_info, hold_info);
    }
}
