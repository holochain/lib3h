use crate::dht::dht_event::{DhtEvent, PeerHoldRequestData};
use lib3h_protocol::{AddressRef, DidWork, Lib3hResult, data_types::EntryData};

/// Allow storage and retrieval of peer info and data
///  + events & fns for gossip communication?
pub trait Dht {
    /// Peer info
    fn get_peer(&self, peer_address: &str) -> Option<PeerHoldRequestData>;
    fn fetch_peer(&self, peer_address: &str) -> Option<PeerHoldRequestData>;
    fn drop_peer(&self, peer_address: &str) -> Lib3hResult<()>;
    fn get_peer_list(&self) -> Vec<PeerHoldRequestData>;
    /// Data
    fn get_entry(&self, entry_address: &AddressRef) -> Option<EntryData>;
    fn fetch_entry(&self, entry_address: &AddressRef) -> Option<EntryData>;
    /// Processing
    fn post(&mut self, evt: DhtEvent) -> Lib3hResult<()>;
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)>;
    /// Getters
    /// Get peer address used on the DHT
    fn this_peer(&self) -> Lib3hResult<&str>;
}
