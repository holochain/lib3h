use lib3h_protocol::uri::Lib3hUri;

pub const DEFAULT_GOSSIP_INTERVAL_MS: u64 = 2000;
pub const DEFAULT_TIMEOUT_THRESHOLD_MS: u64 = 60000;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DhtConfig {
    this_peer_name: Lib3hUri,
    custom: Vec<u8>,
    gossip_interval: u64,
    timeout_threshold: u64,
}

impl DhtConfig {
    pub fn new(peer_name: &Lib3hUri) -> Self {
        DhtConfig {
            this_peer_name: peer_name.to_owned(),
            custom: vec![],
            gossip_interval: DEFAULT_GOSSIP_INTERVAL_MS,
            timeout_threshold: DEFAULT_TIMEOUT_THRESHOLD_MS,
        }
    }

    pub fn with_engine_config(peer_name: &Lib3hUri, config: &crate::engine::EngineConfig) -> Self {
        Self {
            this_peer_name: peer_name.to_owned(),
            custom: config.clone().dht_custom_config,
            gossip_interval: config.dht_gossip_interval,
            timeout_threshold: config.dht_timeout_threshold,
        }
    }

    pub fn timeout_threshold(&self) -> u64 {
        self.timeout_threshold
    }

    pub fn gossip_interval(&self) -> u64 {
        self.gossip_interval
    }

    pub fn this_peer_name(&self) -> Lib3hUri {
        self.this_peer_name.clone()
    }
}
