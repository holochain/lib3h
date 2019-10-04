use crate::{
    dht::dht_protocol::*,
    engine::p2p_protocol::P2pProtocol,
    gateway::{protocol::*, send_data_types::*, GatewayOutputWrapType, P2pGateway},
    message_encoding::encoding_protocol,
    transport,
};
use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::data_types::*;
use rmp_serde::Serializer;
use serde::Serialize;

const SEND_RETRY_INTERVAL_MS: u64 = 20;
const SEND_RETRY_TIMEOUT_MS: u64 = 20000;

/// we want to invoke this on the very next process call
/// set our last_attempt back far enough to ensure this
fn last_attempt_run_on_next_process() -> std::time::Instant {
    std::time::Instant::now()
        .checked_sub(std::time::Duration::from_millis(SEND_RETRY_INTERVAL_MS * 2))
        .expect("can subtract duration")
}

/// Private internals
impl P2pGateway {
    /// check / dispatch all pending sends
    pub(crate) fn process_transport_pending_sends(&mut self) -> GhostResult<()> {
        let mut errors: Vec<GhostError> = Vec::new();

        let meta_list = self.pending_send_queue.drain(..).collect::<Vec<_>>();
        for send_meta in meta_list {
            match self.priv_send_check_dispatch(send_meta) {
                Ok(()) => (),
                Err(e) => errors.push(e),
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.into())
        }
    }

    /// will attempt to resolve uri && pass call to priv_send_with_full_low_uri
    pub(crate) fn send_with_partial_high_uri(
        &mut self,
        send_data: SendWithPartialHighUri,
        cb: SendCallback,
    ) -> GhostResult<()> {
        self.priv_send_with_partial_high_uri(
            send_data,
            std::time::Instant::now()
                .checked_add(std::time::Duration::from_millis(SEND_RETRY_TIMEOUT_MS))
                .expect("can add"),
            cb,
        )
    }

    /// will attempt to send a message with retry given fully qualified uri
    pub(crate) fn send_with_full_low_uri(
        &mut self,
        send_data: SendWithFullLowUri,
        cb: SendCallback,
    ) -> GhostResult<()> {
        self.priv_send_with_full_low_uri(
            send_data,
            std::time::Instant::now()
                .checked_add(std::time::Duration::from_millis(SEND_RETRY_TIMEOUT_MS))
                .expect("can add"),
            cb,
        )
    }

    /// not ready to send this yet, queue for later processing
    fn priv_send_queue_pending(&mut self, send_meta: SendMetaData) -> GhostResult<()> {
        let now = std::time::Instant::now();

        if send_meta.expires_at < now {
            return (send_meta.cb)(Err("timeout".into()));
        }

        self.pending_send_queue.push(send_meta);
        Ok(())
    }

    /// check / dispatch a pending send attempt
    fn priv_send_check_dispatch(&mut self, send_meta: SendMetaData) -> GhostResult<()> {
        let now = std::time::Instant::now();

        if now.duration_since(send_meta.last_attempt)
            < std::time::Duration::from_millis(SEND_RETRY_INTERVAL_MS)
        {
            return self.priv_send_queue_pending(send_meta);
        }

        match send_meta {
            SendMetaData {
                send_data,
                last_attempt: _,
                expires_at,
                cb,
            } => match send_data {
                SendData::WithPartialHighUri(send_data) => {
                    self.priv_send_with_partial_high_uri(send_data, expires_at, cb)
                }
                SendData::WithFullLowUri(send_data) => {
                    self.priv_send_with_full_low_uri(send_data, expires_at, cb)
                }
            },
        }
    }

    /// will attempt to resolve uri && pass call to priv_send_with_full_low_uri
    fn priv_send_with_partial_high_uri(
        &mut self,
        send_data: SendWithPartialHighUri,
        expires_at: std::time::Instant,
        cb: SendCallback,
    ) -> GhostResult<()> {
        // capture this first so our interval doesn't drift too much
        let last_attempt = std::time::Instant::now();

        let uri = send_data.partial_high_uri.clone();
        trace!("want send to {}", uri);

        // ask our inner_dht for the low-level id
        self.inner_dht.request(
            send_data.span.child("dht::RequestPeer"),
            DhtRequestToChild::RequestPeer(uri),
            Box::new(move |me, resp| {
                match resp {
                    GhostCallbackData::Response(Ok(DhtRequestToChildResponse::RequestPeer(
                        Some(peer_data),
                    ))) => {
                        // hey, we got a low-level uri, let's process it
                        let mut uri = peer_data.peer_location.clone();
                        uri.set_agent_id(&peer_data.peer_name.lower_address());
                        trace!("send to {}", uri);
                        me.priv_send_with_full_low_uri(
                            SendWithFullLowUri {
                                span: Span::fixme(),
                                full_low_uri: uri,
                                payload: send_data.payload,
                            },
                            expires_at,
                            cb,
                        )?;
                    }
                    _ => {
                        debug!("could not send {:?}", resp);

                        // could not get low-level uri, try again later
                        me.priv_send_queue_pending(SendMetaData {
                            send_data: SendData::WithPartialHighUri(send_data),
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
    fn priv_send_with_full_low_uri(
        &mut self,
        send_data: SendWithFullLowUri,
        expires_at: std::time::Instant,
        cb: SendCallback,
    ) -> GhostResult<()> {
        // just pass through to the encoder
        self.priv_send_with_full_low_uri_encode(send_data, expires_at, cb)
    }

    /// run an encoding pass on our payload
    fn priv_send_with_full_low_uri_encode(
        &mut self,
        send_data: SendWithFullLowUri,
        expires_at: std::time::Instant,
        cb: SendCallback,
    ) -> GhostResult<()> {
        if !self.message_encoding.is_attached() {
            // we must be in a call chain resulting from an incoming message
            // we need to wait for the next process() call to continue
            return self.priv_send_queue_pending(SendMetaData {
                send_data: SendData::WithFullLowUri(send_data),
                last_attempt: last_attempt_run_on_next_process(),
                expires_at,
                cb,
            });
        }

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
                        me.priv_send_with_full_low_uri_inner(
                            send_data,
                            payload,
                            last_attempt,
                            expires_at,
                            cb,
                        )?;
                    }
                    _ => {
                        me.priv_send_queue_pending(SendMetaData {
                            send_data: SendData::WithFullLowUri(send_data),
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
    fn priv_send_with_full_low_uri_inner(
        &mut self,
        send_data: SendWithFullLowUri,
        encoded_payload: Opaque,
        last_attempt: std::time::Instant,
        expires_at: std::time::Instant,
        cb: SendCallback,
    ) -> GhostResult<()> {
        let to_agent_id = match send_data.full_low_uri.agent_id() {
            Some(agent_id) => agent_id,
            None => "".to_string().into(), // TODO - is this correct?
        };

        let payload =
            if let GatewayOutputWrapType::WrapOutputWithP2pDirectMessage = self.wrap_output_type {
                let dm_wrapper = DirectMessageData {
                    space_address: self.identifier.id.clone(),
                    request_id: "".to_string(),
                    to_agent_id,
                    from_agent_id: self.this_peer.peer_name.clone().into(),
                    content: encoded_payload,
                };
                let mut payload = Vec::new();
                let p2p_msg = P2pProtocol::DirectMessage(dm_wrapper);
                p2p_msg
                    .serialize(&mut Serializer::new(&mut payload))
                    .unwrap();
                Opaque::from(payload)
            } else {
                encoded_payload
            };

        let mut uri = send_data.full_low_uri.clone();

        // at last, we need to drop the high-level agent id
        uri.clear_agent_id();

        self.inner_transport.request(
            Span::fixme(),
            transport::protocol::RequestToChild::SendMessage { uri, payload },
            Box::new(move |me, resp| {
                match resp {
                    GhostCallbackData::Response(Ok(
                        transport::protocol::RequestToChildResponse::SendMessageSuccess,
                    )) => {
                        cb(Ok(GatewayRequestToChildResponse::Transport(
                            transport::protocol::RequestToChildResponse::SendMessageSuccess,
                        )))?;
                    }
                    _ => {
                        me.priv_send_queue_pending(SendMetaData {
                            send_data: SendData::WithFullLowUri(send_data),
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
}
