pub mod ghost_engine;
pub mod ghost_engine_wrapper;
mod network_layer;
pub mod p2p_protocol;
pub mod real_engine;
mod space_layer;

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    dht::dht_protocol::*,
    gateway::{protocol::*, GatewayUserData},
    track::Tracker,
    transport::TransportMultiplex,
};
use detach::prelude::*;
use lib3h_crypto_api::{Buffer, CryptoSystem};
use lib3h_protocol::{
    protocol_client::Lib3hClientProtocol, protocol_server::Lib3hServerProtocol, Address,
};
use lib3h_tracing::Lib3hTrace;
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serializer};
use url::Url;

/// Identifier of a source chain: SpaceAddress+AgentId
pub type ChainId = (Address, Address);

pub static NETWORK_GATEWAY_ID: &'static str = "__network__";

fn vec_url_de<'de, D>(deserializer: D) -> Result<Vec<Url>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper(#[serde(with = "url_serde")] Url);

    let v = Vec::deserialize(deserializer)?;
    Ok(v.into_iter().map(|Wrapper(a)| a).collect())
}

fn vec_url_se<S>(v: &[Url], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    #[derive(Serialize)]
    struct Wrapper(#[serde(with = "url_serde")] Url);

    let mut seq = serializer.serialize_seq(Some(v.len()))?;
    for u in v {
        seq.serialize_element(&Wrapper(u.clone()))?;
    }
    seq.end()
}

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
    #[serde(deserialize_with = "vec_url_de", serialize_with = "vec_url_se")]
    pub bootstrap_nodes: Vec<Url>,
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
pub struct RealEngine {
    /// Identifier
    name: String,
    /// Config settings
    config: RealEngineConfig,
    /// FIFO of Lib3hClientProtocol messages received from Core
    inbox: VecDeque<Lib3hClientProtocol>,
    /// Factory for building the DHTs used by the gateways
    dht_factory: DhtFactory,
    /// Tracking request_id's sent to core
    request_track: Tracker<RealEngineTrackerData>,

    multiplexer: TransportMultiplex,
    // Should be owned by multiplexer
    // TODO #176: Remove this if we resolve #176 without it.
    /// P2p gateway for the network layer
    network_gateway: Detach<GatewayParentWrapperDyn<RealEngine, Lib3hTrace>>,

    /// Cached this_peer of the network_gateway
    this_net_peer: PeerData,

    /// Store active connections?
    network_connections: HashSet<Url>,

    /// Map of P2p gateway per Space+Agent
    space_gateway_map: HashMap<ChainId, Detach<GatewayParentWrapperDyn<RealEngine, Lib3hTrace>>>,
    #[allow(dead_code)]
    /// crypto system to use
    crypto: Box<dyn CryptoSystem>,
    #[allow(dead_code)]
    /// transport_id data, public/private keys, etc
    transport_keys: TransportKeys,
    /// debug: count number of calls to process()
    process_count: u64,
    /// dht ghost user_data
    /// temp HACK. Waiting for gateway actor
    temp_outbox: Vec<Lib3hServerProtocol>,

    // user data for ghost callback
    gateway_user_data: GatewayUserData,
}
