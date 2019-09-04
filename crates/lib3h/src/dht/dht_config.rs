use crate::dht::PeerAddress;
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
