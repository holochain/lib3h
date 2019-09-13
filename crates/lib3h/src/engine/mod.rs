pub mod ghost_engine;
pub mod ghost_engine_wrapper;
mod network_layer;
pub mod p2p_protocol;
mod space_layer;

use crate::{
    dht::dht_protocol::*,
    error::Lib3hError,
    gateway::{protocol::*, P2pGateway},
    track::Tracker,
    transport::TransportMultiplex,
};
use detach::Detach;
use lib3h_crypto_api::{Buffer, CryptoSystem};
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{protocol::*, Address};
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serializer};
use std::collections::{HashMap, HashSet};
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
#[allow(dead_code)]
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
pub struct EngineConfig {
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

#[allow(dead_code)]
pub struct GhostEngine<'engine> {
    /// Identifier
    name: String,
    /// Config setting
    config: EngineConfig,
    /// Factory for building DHT's of type D
    dht_factory: DhtFactory,
    /// Tracking request_id's sent to core
    request_track: Tracker<RealEngineTrackerData>,
    /// P2p gateway for the network layer
    multiplexer: Detach<GatewayParentWrapper<GhostEngine<'engine>, TransportMultiplex<P2pGateway>>>,
    /// Cached this_peer of the multiplexer
    this_net_peer: PeerData,

    /// Store active connections?
    network_connections: HashSet<Url>,
    /// Map of P2p gateway per Space+Agent
    space_gateway_map:
        HashMap<ChainId, Detach<GatewayParentWrapper<GhostEngine<'engine>, P2pGateway>>>,
    #[allow(dead_code)]
    /// crypto system to use
    crypto: Box<dyn CryptoSystem>,
    #[allow(dead_code)]
    /// transport_id data, public/private keys, etc
    transport_keys: TransportKeys,
    /// debug: count number of calls to process()
    process_count: u64,

    client_endpoint: Option<
        GhostEndpoint<
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hToClient,
            Lib3hToClientResponse,
            Lib3hError,
        >,
    >,
    lib3h_endpoint: Detach<
        GhostContextEndpoint<
            GhostEngine<'engine>,
            Lib3hToClient,
            Lib3hToClientResponse,
            ClientToLib3h,
            ClientToLib3hResponse,
            Lib3hError,
        >,
    >,
}
