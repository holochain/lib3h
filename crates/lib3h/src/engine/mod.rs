mod network_layer;
pub mod p2p_protocol;
pub mod real_engine;
mod space_layer;

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    dht::dht_trait::{Dht, DhtFactory},
    track::Tracker,
    transport::protocol::TransportParentWrapper,
};
use detach::prelude::*;
use lib3h_crypto_api::{Buffer, CryptoSystem};
use lib3h_protocol::{protocol_client::Lib3hClientProtocol, Address};
use url::Url;

/// Identifier of a source chain: SpaceAddress+AgentId
pub type ChainId = (Address, Address);

pub static NETWORK_GATEWAY_ID: &'static str = "__network__";

#[derive(Debug, Clone, PartialEq, Eq)]
enum RealEngineTrackerData {
    /// track the actual HandleGetGossipingEntryList request
    GetGossipingEntryList,
    /// track the actual HandleGetAuthoringEntryList request
    GetAuthoringEntryList,
    /// once we have the AuthoringEntryListResponse, fetch data for entries
    DataForAuthorEntry,
    /// gossip has requested we store data, send a hold request to core
    /// core should respond ??
    HoldEntryRequested,
}

/// Struct holding all config settings for the RealEngine
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RealEngineConfig {
    //pub tls_config: TlsConfig,
    pub socket_type: String,
    pub bootstrap_nodes: Vec<String>,
    pub work_dir: String,
    pub log_level: char,
    #[serde(with = "url_serde")]
    pub bind_url: Url,
    pub dht_gossip_interval: u64,
    pub dht_timeout_threshold: u64,
    pub dht_custom_config: Vec<u8>,
}

pub struct TransportKeys {
    /// Our TransportId, i.e. Base32 encoded public key (e.g. "HcMyadayada")
    pub transport_id: String,
    /// The TransportId public key
    pub transport_public_key: Box<dyn Buffer>,
    /// The TransportId secret key
    pub transport_secret_key: Box<dyn Buffer>,
}

/// Lib3h's 'real mode' as a NetworkEngine
pub struct RealEngine<D: Dht> {
    /// Identifier
    name: String,
    /// Config settings
    config: RealEngineConfig,
    /// FIFO of Lib3hClientProtocol messages received from Core
    inbox: VecDeque<Lib3hClientProtocol>,
    /// Factory for building DHT's of type D
    dht_factory: DhtFactory<D>,
    /// Tracking request_id's sent to core
    request_track: Tracker<RealEngineTrackerData>,
    // TODO #176: Remove this if we resolve #176 without it.
    #[allow(dead_code)]
    /// Transport used by the network gateway
    //network_transport: TransportWrapper<'engine>,
    /// P2p gateway for the network layer
    network_gateway: Detach<TransportParentWrapper>,
    /// Store active connections?
    network_connections: HashSet<Url>,
    /// Map of P2p gateway per Space+Agent
    space_gateway_map: HashMap<ChainId, TransportParentWrapper>,
    #[allow(dead_code)]
    /// crypto system to use
    crypto: Box<dyn CryptoSystem>,
    #[allow(dead_code)]
    /// transport_id data, public/private keys, etc
    transport_keys: TransportKeys,
    /// debug: count number of calls to process()
    process_count: u64,
    this_peer_address: Url, // binding result on the network_gateway
}
