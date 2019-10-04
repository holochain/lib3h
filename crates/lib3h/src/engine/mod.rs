pub mod engine_actor;
pub mod ghost_engine;
pub mod ghost_engine_wrapper;
mod network_layer;
pub mod p2p_protocol;
mod space_layer;

use crate::{
    dht::dht_protocol::*,
    engine::engine_actor::ClientToLib3hMessage,
    error::*,
    gateway::{protocol::*, P2pGateway},
    track::Tracker,
    transport::{websocket::tls::TlsConfig, TransportMultiplex},
};
use detach::Detach;
use lib3h_crypto_api::{Buffer, CryptoSystem};
use lib3h_ghost_actor::{prelude::*, RequestId};
use lib3h_protocol::{protocol::*, types::SpaceHash, uri::Lib3hUri, Address};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

/// Identifier of a source chain: SpaceAddress+AgentId
pub type ChainId = (SpaceHash, Address);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GatewayId {
    pub nickname: String,
    pub id: Address,
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

/// Transport specific configuration
/// NB: must be externally tagged because that is the only way that
/// tuple struct variants can be serialized to TOML
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "data")]
pub enum TransportConfig {
    Websocket(TlsConfig),
    Memory(String),
}

/// Struct holding all config settings for the Engine
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EngineConfig {
    pub network_id: GatewayId,
    pub transport_configs: Vec<TransportConfig>,
    pub bootstrap_nodes: Vec<Lib3hUri>,
    pub work_dir: PathBuf,
    pub log_level: char,
    pub bind_url: Lib3hUri,
    pub dht_gossip_interval: u64,
    pub dht_timeout_threshold: u64,
    pub dht_custom_config: Vec<u8>,
}

pub struct TransportKeys {
    /// Our TransportId, i.e. Base32 encoded public key (e.g. "HcMyadayada")
    pub transport_id: Address,
    /// The TransportId public key
    pub transport_public_key: Box<dyn Buffer>,
    /// The TransportId secret key
    pub transport_secret_key: Box<dyn Buffer>,
}
impl TransportKeys {
    pub fn new(crypto: &dyn CryptoSystem) -> Lib3hResult<Self> {
        let hcm0 = hcid::HcidEncoding::with_kind("hcm0")?;
        let mut public_key: Box<dyn Buffer> = Box::new(vec![0; crypto.sign_public_key_bytes()]);
        let mut secret_key = crypto.buf_new_secure(crypto.sign_secret_key_bytes());
        crypto.sign_keypair(&mut public_key, &mut secret_key)?;
        Ok(Self {
            transport_id: hcm0.encode(&public_key)?.into(),
            transport_public_key: public_key,
            transport_secret_key: secret_key,
        })
    }
}

pub trait CanAdvertise {
    fn advertise(&self) -> Lib3hUri;
}

pub struct GhostEngine<'engine> {
    /// Identifier
    name: String,
    /// Config settings
    config: EngineConfig,
    /// Factory for building the DHTs used by the gateways
    dht_factory: DhtFactory,
    /// Tracking request_id's sent to core
    request_track: Tracker<RealEngineTrackerData>,
    /// Multiplexer holding the network gateway
    multiplexer: Detach<GatewayParentWrapper<GhostEngine<'engine>, TransportMultiplex<P2pGateway>>>,
    /// Cached this_peer of the multiplexer
    this_net_peer: PeerData,

    /// Store active connections?
    network_connections: HashSet<Lib3hUri>,
    /// Map of P2p gateway per Space+Agent
    space_gateway_map:
        HashMap<ChainId, Detach<GatewayParentWrapper<GhostEngine<'engine>, P2pGateway>>>,
    #[allow(dead_code)]
    /// crypto system to use
    crypto: Box<dyn CryptoSystem>,
    #[allow(dead_code)]
    /// transport_id data, public/private keys, etc
    transport_keys: TransportKeys,
    /// items we need to send on our multiplexer in another process loop
    multiplexer_defered_sends: Vec<(Lib3hUri, lib3h_protocol::data_types::Opaque)>,

    /// when client gives us a SendDirectMessage, we need to cache the
    /// GhostMessage, re-hydrate when a response comes back from a remote
    pending_client_direct_messages: HashMap<RequestId, ClientToLib3hMessage>,

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
