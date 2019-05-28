use crate::dht::dht_event::{DhtEvent, PeerHoldRequestData};
use lib3h_protocol::{AddressRef, DidWork, Lib3hResult};

/// Allow storage and retrieval of peer info and data
///  + events & fns for gossip communication?
pub trait Dht {
    /// Peer info
    fn get_peer(&self, peer_address: &str) -> Option<PeerHoldRequestData>;
    fn fetch_peer(&self, peer_address: &str) -> Option<PeerHoldRequestData>;
    fn drop_peer(&self, peer_address: &str) -> Lib3hResult<()>;
    /// Data
    fn get_data(&self, data_address: &AddressRef) -> Lib3hResult<Vec<u8>>;
    fn fetch_data(&self, data_address: &AddressRef) -> Lib3hResult<Vec<u8>>;
    /// Processing
    fn post(&mut self, evt: DhtEvent) -> Lib3hResult<()>;
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)>;
    /// Getters
    fn this_peer(&self) -> Lib3hResult<()>;
}
