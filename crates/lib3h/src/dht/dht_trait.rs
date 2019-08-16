use crate::{
    dht::{
        dht_protocol::{DhtCommand, DhtEvent, PeerData},
        PeerAddress, PeerAddressRef,
    },
    error::Lib3hResult,
};
use lib3h_protocol::{Address, DidWork};
use url::Url;

pub const DEFAULT_GOSSIP_INTERVAL_MS: u64 = 1000;
pub const DEFAULT_TIMEOUT_THRESHOLD_MS: u64 = 60000;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DhtConfig {
    pub this_peer_address: PeerAddress,
    #[serde(with = "url_serde")]
    pub this_peer_uri: Url,
    pub custom: Vec<u8>,
    pub gossip_interval: u64,
    pub timeout_threshold: u64,
}

impl DhtConfig {
    pub fn new(peer_address: &str, peer_uri: &Url) -> Self {
        DhtConfig {
            this_peer_address: peer_address.to_owned(),
            this_peer_uri: peer_uri.to_owned(),
            custom: vec![],
            gossip_interval: DEFAULT_GOSSIP_INTERVAL_MS,
            timeout_threshold: DEFAULT_TIMEOUT_THRESHOLD_MS,
        }
    }
}
pub type DhtFactory<D> = fn(config: &DhtConfig) -> Lib3hResult<D>;

/// Allow storage and retrieval of peer & entry data.
/// Trait API is for querying local dht data
/// DhtCommand is for mutating storage or network/async queries
pub trait Dht {
    /// Peer info
    fn get_peer_list(&self) -> Vec<PeerData>;
    fn get_peer(&self, peer_address: &PeerAddressRef) -> Option<PeerData>;
    fn get_peer_by_uri(&self, peer_uri: &Url) -> Option<PeerData>;
    fn this_peer(&self) -> &PeerData;
    /// Entry
    fn get_entry_address_list(&self) -> Vec<&Address>;
    fn get_aspects_of(&self, entry_address: &Address) -> Option<Vec<Address>>;
    /// Processing
    fn post(&mut self, cmd: DhtCommand) -> Lib3hResult<()>;
    fn process(&mut self) -> Lib3hResult<(DidWork, Vec<DhtEvent>)>;
}
