#[allow(non_snake_case)]
pub mod gateway_actor;
pub mod gateway_dht;
pub mod gateway_transport;
pub mod gateway_transport_send;
pub mod p2p_gateway;
pub mod protocol;

use crate::{
    dht::dht_protocol::*,
    engine::GatewayId,
    gateway::protocol::*,
    message_encoding::*,
    transport::{self, error::TransportResult},
};

use detach::prelude::*;
use holochain_tracing::Span;
use lib3h_ghost_actor::GhostResult;
use lib3h_protocol::{data_types::Opaque, uri::Lib3hUri};
use std::boxed::Box;

pub enum GatewayOutputWrapType {
    DoNotWrapOutput,
    WrapOutputWithP2pDirectMessage,
}

/// Combines a Transport and a DHT.
/// Tracks distributed data for that P2P network in a DHT.
pub struct P2pGateway {
    wrap_output_type: GatewayOutputWrapType,

    // either network_id or space_address depending on which type of gateway
    identifier: GatewayId,

    /// Transport
    inner_transport: Detach<transport::protocol::TransportActorParentWrapperDyn<Self>>,
    /// DHT
    inner_dht: Detach<ChildDhtWrapperDyn<P2pGateway>>,

    /// message encoding actor
    message_encoding: Detach<MessageEncodingActorParentWrapper<P2pGateway>>,

    /// self ghost actor
    endpoint_parent: Option<GatewayParentEndpoint>,
    endpoint_self: Detach<GatewaySelfEndpoint<()>>,
    /// cached data from inner dht
    this_peer: PeerData,

    pending_send_queue: Vec<send_data_types::SendMetaData>,
}

pub(crate) mod send_data_types {
    use super::*;

    #[derive(Debug)]
    /// we have a partial address for the remote... we know their id
    /// but we do not know their low-level uri
    pub(crate) struct SendWithPartialHighUri {
        pub span: Span,
        pub partial_high_uri: Lib3hUri,
        pub payload: Opaque,
    }

    #[derive(Debug)]
    /// we have already resolved the low-level uri
    /// meaning we have a fully qualified address for the remote
    pub(crate) struct SendWithFullLowUri {
        pub span: Span,
        pub full_low_uri: Lib3hUri,
        pub payload: Opaque,
    }

    #[derive(Debug)]
    /// allows grouping these with metadata below
    pub(crate) enum SendData {
        WithPartialHighUri(SendWithPartialHighUri),
        WithFullLowUri(SendWithFullLowUri),
    }

    /// metadata associated with send retry tracking
    pub(crate) struct SendMetaData {
        pub send_data: SendData,
        pub last_attempt: std::time::Instant,
        pub expires_at: std::time::Instant,
        pub cb: SendCallback,
    }

    impl std::fmt::Debug for SendMetaData {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SendMetaData")
                .field("send_data", &self.send_data)
                .field("last_attempt", &self.last_attempt)
                .field("expires_at", &self.expires_at)
                .field("cb", &"SendCallback { ... }".to_string())
                .finish()
        }
    }

    /// internal callback type for send results
    pub(crate) type SendCallback =
        Box<dyn FnOnce(TransportResult<GatewayRequestToChildResponse>) -> GhostResult<()> + 'static>;
}

/*
#[derive(Debug)]
struct PendingOutgoingMessage {
    span: Span,
    uri: Lib3hUri,
    payload: Opaque,
    parent_request: GatewayToChildMessage,
    attempt: u8,
}
*/
