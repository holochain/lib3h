use crate::dht::dht_protocol::{DhtCommand, DhtEvent, PeerData};
use lib3h_protocol::{Address, DidWork, Lib3hResult};
use url::Url;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DhtConfig {
    pub this_peer_address: String,
    #[serde(with = "url_serde")]
    pub this_peer_uri: Url,
    pub custom: Vec<u8>,
    pub gossip_interval: u64,
    pub timeout_threshold: u64,
}

pub type DhtFactory<D> = fn(config: &DhtConfig) -> Lib3hResult<D>;

/// Allow storage and retrieval of peer & entry data.
/// Trait API is for querying local dht data
/// DhtCommand is for mutating storage or network/async queries
pub trait Dht {
    /// Peer info
    fn get_peer_list(&self) -> Vec<PeerData>;
    fn get_peer(&self, peer_address: &str) -> Option<PeerData>;
    fn this_peer(&self) -> &PeerData;
    /// Entry
    fn get_entry_address_list(&self) -> Vec<&Address>;
    fn get_aspects_of(&self, entry_address: &Address) -> Option<Vec<Address>>;
    /// Processing
    fn post(&mut self, cmd: DhtCommand) -> Lib3hResult<()>;
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)>;
}
