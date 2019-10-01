use crate::{
    dht::dht_protocol::*,
    engine::p2p_protocol::P2pProtocol,
    error::*,
    gateway::{
        protocol::*, GatewayOutputWrapType, P2pGateway, PendingOutgoingMessage, SendCallback,
    },
    message_encoding::encoding_protocol,
    transport::{self, error::TransportResult},
};
use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::*, uri::Lib3hUri, Address};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

const SEND_RETRY_INTERVAL_MS: u64 = 200;
const SEND_RETRY_TIMEOUT_MS: u64 = 2000;

/// we have a partial address for the remote... we know their id
/// but we do not know their low-level uri
pub(crate) struct SendWithPartialHighId {
    pub span: Span,
    pub partial_high_uri: Lib3hUri,
    pub payload: Opaque,
}

/// we have already resolved the low-level uri
/// meaning we have a fully qualified address for the remote
pub(crate) struct SendWithFullLowId {
    pub span: Span,
    pub full_low_uri: Lib3hUri,
    pub payload: Opaque,
}

/// allows grouping these with metadata below
enum SendData {
    WithPartialHighId(SendWithPartialHighId),
    WithFullLowId(SendWithFullLowId),
}

/// metadata associated with send retry tracking
struct SendMetaData {
    pub send_data: SendData,
    pub last_attempt: std::time::Instant,
    pub expires_at: std::time::Instant,
    pub cb: SendCallback,
}

/// Private internals
impl P2pGateway {
    /// will attempt to resolve uri && pass call to priv_send_with_full_low_id
    pub(crate) fn send_with_partial_high_id(&mut self, send_data: SendWithPartialHighId, cb: SendCallback) -> GhostResult<()> {
        self.priv_send_with_partial_high_id(
            send_data,
            std::time::Instant::now().checked_add(
                std::time::Duration::from_millis(SEND_RETRY_TIMEOUT_MS)
            ).expect("can add"),
            cb,
        )
    }

    /// will attempt to send a message with retry given fully qualified uri
    pub(crate) fn send_with_full_low_id(&mut self, send_data: SendWithFullLowId, cb: SendCallback) -> GhostResult<()> {
        self.priv_send_with_full_low_id(
            send_data,
            std::time::Instant::now().checked_add(
                std::time::Duration::from_millis(SEND_RETRY_TIMEOUT_MS)
            ).expect("can add"),
            cb,
        )
    }

    fn priv_send_queue_pending(&mut self, send_meta: SendMetaData) -> GhostResult<()> {
        let now = std::time::Instant::now();

        if send_meta.expires_at < now {
            return (send_meta.cb)(Err("timeout".into()));
        }

        unimplemented!();
    }

    /// check / dispatch a pending send attempt
    fn priv_send_check_dispatch(&mut self, send_meta: SendMetaData) -> GhostResult<()> {
        let now = std::time::Instant::now();

        if now.duration_since(send_meta.last_attempt) < std::time::Duration::from_millis(SEND_RETRY_INTERVAL_MS) {
            self.priv_send_queue_pending(send_meta);
            return Ok(());
        }

        match send_meta {
            SendMetaData { send_data, last_attempt, expires_at, cb } => {
                match send_data {
                    SendData::WithPartialHighId(send_data) => {
                        self.priv_send_with_partial_high_id(send_data, expires_at, cb)
                    }
                    SendData::WithFullLowId(send_data) => {
                        self.priv_send_with_full_low_id(send_data, expires_at, cb)
                    }
                }
            }
        }
    }

    /// will attempt to resolve uri && pass call to priv_send_with_full_low_id
    fn priv_send_with_partial_high_id(&mut self, send_data: SendWithPartialHighId, expires_at: std::time::Instant, cb: SendCallback) -> GhostResult<()> {
        // capture this first so our interval doesn't drift too much
        let last_attempt = std::time::Instant::now();

        let uri = send_data.partial_high_uri.clone();

        // ask our inner_dht for the low-level id
        self.inner_dht.request(
            send_data.span.child("dht::RequestPeer"),
            DhtRequestToChild::RequestPeer(uri),
            Box::new(move |me, response| {
                match response {
                    GhostCallbackData::Response(Ok(
                        DhtRequestToChildResponse::RequestPeer(Some(peer_data))
                    )) => {
                        // hey, we got a low-level uri, let's process it
                        let uri = peer_data.peer_location.clone();
                        error!("IS THIS FULLY QUALIFIED?? {:?}", uri);
                        me.priv_send_with_full_low_id(SendWithFullLowId {
                            span: Span::fixme(),
                            full_low_uri: uri,
                            payload: send_data.payload,
                        }, expires_at, cb)?;
                    }
                    _ => {
                        // could not get low-level uri, try again later
                        me.priv_send_queue_pending(SendMetaData {
                            send_data: SendData::WithPartialHighId(send_data),
                            last_attempt,
                            expires_at,
                            cb,
                        })?;
                    }
                }
                Ok(())
            }),
        )?;
        Ok(())
    }

    /// will attempt to send a message with retry given fully qualified uri
    fn priv_send_with_full_low_id(&mut self, send_data: SendWithFullLowId, expires_at: std::time::Instant, cb: SendCallback) -> GhostResult<()> {
        // just pass through to the encoder
        self.priv_send_with_full_low_id_encode(send_data, expires_at, cb)
    }

    /// run an encoding pass on our payload
    fn priv_send_with_full_low_id_encode(&mut self, send_data: SendWithFullLowId, expires_at: std::time::Instant, cb: SendCallback) -> GhostResult<()> {
        // capture this first so our interval doesn't drift too much
        let last_attempt = std::time::Instant::now();

        let payload = send_data.payload.clone();

        self.message_encoding.request(
            Span::fixme(),
            encoding_protocol::RequestToChild::EncodePayload { payload },
            Box::new(move |me, resp| {
                match resp {
                    GhostCallbackData::Response(Ok(
                        encoding_protocol::RequestToChildResponse::EncodePayloadResult { payload },
                    )) => {
                        me.priv_send_with_full_low_id_inner(send_data, payload, last_attempt, expires_at, cb)?;
                    }
                    _ => {
                        me.priv_send_queue_pending(SendMetaData {
                            send_data: SendData::WithFullLowId(send_data),
                            last_attempt,
                            expires_at,
                            cb,
                        })?;
                    }
                }
                Ok(())
            }),
        )
    }

    /// finally, actually send the message out our inner transport
    fn priv_send_with_full_low_id_inner(&mut self, send_data: SendWithFullLowId, encoded_payload: Opaque, last_attempt: std::time::Instant, expires_at: std::time::Instant, cb: SendCallback) -> GhostResult<()> {
        Ok(())
    }
}
