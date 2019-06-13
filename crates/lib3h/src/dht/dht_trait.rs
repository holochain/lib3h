use crate::dht::dht_protocol::{DhtCommand, DhtEvent, PeerData};
use lib3h_protocol::{data_types::EntryData, AddressRef, DidWork, Lib3hResult};

/// Allow storage and retrieval of peer & entry data.
/// Trait API is for query
/// DhtEvent is for mutating storage
pub trait Dht {
    /// Peer info
    fn get_peer(&self, peer_address: &str) -> Option<PeerData>;
    fn fetch_peer(&self, peer_address: &str) -> Option<PeerData>;
    fn get_peer_list(&self) -> Vec<PeerData>;
    fn this_peer(&self) -> Lib3hResult<&str>;
    /// Data
    fn get_entry(&self, entry_address: &AddressRef) -> Option<EntryData>;
    fn fetch_entry(&self, entry_address: &AddressRef) -> Option<EntryData>;
    /// Processing
    fn post(&mut self, cmd: DhtCommand) -> Lib3hResult<()>;
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)>;
}
