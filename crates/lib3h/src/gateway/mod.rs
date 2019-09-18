#[allow(non_snake_case)]
pub mod gateway_actor;
pub mod gateway_dht;
pub mod gateway_transport;
pub mod p2p_gateway;
pub mod protocol;

use crate::{dht::dht_protocol::*, gateway::protocol::*, transport};
use detach::prelude::*;
use lib3h_protocol::data_types::Opaque;
use url::Url;
use lib3h_tracing::Lib3hSpan;

/// Combines a Transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
pub struct P2pGateway {
    /// Used for distinguishing gateways
    identifier: String,

    /// Transport
    inner_transport: Detach<transport::protocol::TransportActorParentWrapperDyn<Self>>,
    /// DHT
    inner_dht: Detach<ChildDhtWrapperDyn<P2pGateway>>,

    /// self ghost actor
    endpoint_parent: Option<GatewayParentEndpoint>,
    endpoint_self: Detach<GatewaySelfEndpoint<()>>,
    /// cached data from inner dht
    this_peer: PeerData,

    pending_outgoing_messages: Vec<PendingOutgoingMessage>,
}

struct PendingOutgoingMessage {
    span: Lib3hSpan,
    uri: Url,
    payload: Opaque,
    parent_msg: GatewayToChildMessage,
}