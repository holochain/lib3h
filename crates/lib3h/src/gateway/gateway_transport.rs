#![allow(non_snake_case)]

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
use lib3h_protocol::{data_types::*, uri::Lib3hUri};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

/// Private internals
impl P2pGateway {
    /// Handle IncomingConnection event from child transport
    fn handle_incoming_connection(&mut self, span: Span, uri: Lib3hUri) -> TransportResult<()> {
        self.inner_dht.request(
            span.child("handle_incoming_connection"),
            DhtRequestToChild::RequestThisPeer,
            Box::new(move |me, response| {
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => match response {
                            Err(e) => panic!("{:?}", e),
                            Ok(response) => response,
                        },
                    }
                };
                if let DhtRequestToChildResponse::RequestThisPeer(this_peer) = response {
                    // Send to other node our PeerName
                    let our_peer_name = P2pProtocol::PeerName(
                        me.identifier.id.to_owned().into(),
                        this_peer.peer_name,
                        this_peer.timestamp,
                    );
                    let mut buf = Vec::new();
                    our_peer_name
                        .serialize(&mut Serializer::new(&mut buf))
                        .unwrap();
                    trace!(
                        "({}) sending P2pProtocol::PeerName: {:?} to {:?}",
                        me.identifier.nickname,
                        our_peer_name,
                        uri,
                    );
                    me.send(
                        span.follower("TODO send"),
                        // This is a little awkward. If we are in wrapping
                        // mode, we still need this to be wrapped... but
                        // the remote side will intercept this message before
                        // it is sent up the chain, so it's ok this is blank.
                        "".to_string().into(),
                        uri.clone(),
                        buf.into(),
                        Box::new(|response| {
                            match response {
                                Ok(GatewayRequestToChildResponse::Transport(
                                    transport::protocol::RequestToChildResponse::SendMessageSuccess,
                                )) => {
                                    trace!("Successfully exchanged peer info with new connection")
                                }
                                _ => error!(
                                    "peer info exchange with new connection failed {:?}",
                                    response
                                ),
                            }
                            Ok(())
                        }),
                    )?;
                } else {
                    panic!("bad response to RequestThisPeer: {:?}", response);
                }
                Ok(())
            }),
        )?;
        Ok(())
    }

    fn priv_decode_on_receive(&mut self, span: Span, uri: Url, payload: Opaque) -> GhostResult<()> {
        let e_span = span.child("on_receive");
        self.message_encoding.request(
            span,
            encoding_protocol::RequestToChild::Decode { payload },
            Box::new(move |me, resp| {
                match resp {
                    GhostCallbackData::Response(Ok(
                        encoding_protocol::RequestToChildResponse::DecodeResult {
                            result: encoding_protocol::DecodeData::Payload { payload },
                        },
                    )) => {
                        if payload.len() == 0 {
                            debug!("Implement Ping!");
                        } else {
                            me.priv_on_receive(e_span, uri, payload)?;
                        }
                    }
                    _ => panic!("unexpected decode result: {:?}", resp),
                }
                Ok(())
            }),
        )
    }

    fn priv_on_receive(&mut self, span: Span, uri: Url, payload: Opaque) -> GhostResult<()> {
        let mut de = Deserializer::new(&payload[..]);
        let maybe_p2p_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
            Deserialize::deserialize(&mut de);

        match maybe_p2p_msg {
            Ok(P2pProtocol::PeerAddress(gateway_id, peer_address, timestamp)) => {
                debug!(
                    "Received PeerAddress: {} | {} ({})",
                    peer_address, gateway_id, self.identifier.nickname
                );
                if self.identifier.id == gateway_id.into() {
                    let peer = PeerData {
                        peer_address,
                        peer_uri: uri.clone(),
                        timestamp,
                    };
                    // HACK
                    self.inner_dht.publish(
                        span.follower("transport::protocol::RequestToParent::ReceivedData"),
                        DhtRequestToChild::HoldPeer(peer),
                    )?;
                }
            }
            Ok(_) => {
                // TODO XXX - nope!
                // We should handle these cases, and pick the ones we want to
                // send up the chain, and which ones should be handled here.

                trace!(
                    "{:?} received {} {}",
                    self.identifier,
                    uri,
                    String::from_utf8_lossy(&payload)
                );

                self.endpoint_self.as_mut().publish(
                    span.follower("bubble up to parent"),
                    GatewayRequestToParent::Transport(
                        transport::protocol::RequestToParent::ReceivedData { uri, payload },
                    ),
                )?;
            }
            _ => {
                panic!(
                    "unexpected received data type {} {:?}",
                    payload, maybe_p2p_msg
                );
            }
        };
        Ok(())
    }

    fn priv_encoded_send(
        &mut self,
        span: Span,
        to_address: lib3h_protocol::Address,
        uri: Lib3hUri,
        payload: Opaque,
        cb: SendCallback,
    ) -> GhostResult<()> {
        let e_span = span.child("encode_payload");
        self.message_encoding.request(
            span,
            encoding_protocol::RequestToChild::EncodePayload { payload },
            Box::new(move |me, resp| {
                match resp {
                    GhostCallbackData::Response(Ok(
                        encoding_protocol::RequestToChildResponse::EncodePayloadResult { payload },
                    )) => {
                        trace!("sending: {:?}", payload);
                        me.priv_low_level_send(e_span, to_address, uri, payload, cb)?;
                    }
                    _ => {
                        cb(Err(format!(
                            "gateway_transport::priv_encoded_send: {:?}",
                            resp
                        )
                        .into()))?;
                    }
                }
                Ok(())
            }),
        )
    }

    fn priv_low_level_send(
        &mut self,
        span: Span,
        to_address: lib3h_protocol::Address,
        uri: Url,
        payload: Opaque,
        cb: SendCallback,
    ) -> GhostResult<()> {
        let payload =
            if let GatewayOutputWrapType::WrapOutputWithP2pDirectMessage = self.wrap_output_type {
                let dm_wrapper = DirectMessageData {
                    space_address: self.identifier.id.clone(),
                    request_id: "".to_string(),
                    to_agent_id: to_address,
                    from_agent_id: self.this_peer.peer_address.clone().into(),
                    content: payload,
                };
                let mut payload = Vec::new();
                let p2p_msg = P2pProtocol::DirectMessage(dm_wrapper);
                p2p_msg
                    .serialize(&mut Serializer::new(&mut payload))
                    .unwrap();
                Opaque::from(payload)
            } else {
                payload
            };

        // Forward to the child Transport
        self.inner_transport.request(
            span.child("SendMessage"),
            transport::protocol::RequestToChild::SendMessage {
                uri: uri.clone(),
                payload: payload,
            },
            Box::new(move |_me, response| {
                // In case of a transport error or timeout we store the message in the
                // pending list to retry sending it later
                match response {
                    // Success case:
                    GhostCallbackData::Response(Ok(
                        transport::protocol::RequestToChildResponse::SendMessageSuccess,
                    )) => {
                        debug!("Gateway send message successfully");
                        cb(Ok(GatewayRequestToChildResponse::Transport(
                            transport::protocol::RequestToChildResponse::SendMessageSuccess,
                        )))
                    }
                    // No error but something other than SendMessageSuccess:
                    GhostCallbackData::Response(Ok(_)) => {
                        warn!(
                            "Gateway got bad response type from transport: {:?}",
                            response
                        );
                        cb(Err(format!("bad response type: {:?}", response).into()))
                    }
                    // Transport error:
                    GhostCallbackData::Response(Err(error)) => {
                        debug!("Gateway got error from transport. Adding message to pending");
                        cb(Err(format!(
                            "Transport error while trying to send message: {:?}",
                            error
                        )
                        .into()))
                    }
                    // Timeout:
                    GhostCallbackData::Timeout => {
                        debug!("Gateway got timeout from transport. Adding message to pending");
                        cb(Err(
                            "Ghost timeout error while trying to send message".into()
                        ))
                    }
                }
            }),
        )
    }

    /// uri =
    ///   - Network : transportId
    ///   - space   : agentId
    pub(crate) fn send(
        &mut self,
        span: Span,
        to_address: lib3h_protocol::Address,
        uri: Url,
        payload: Opaque,
        cb: SendCallback,
    ) -> GhostResult<()> {
        trace!(
            "({}).send() {} | {}",
            self.identifier.nickname,
            uri,
            payload.len()
        );
        self.priv_encoded_send(span, to_address, uri, payload, cb)
    }

    pub(crate) fn handle_transport_pending_outgoing_messages(&mut self) -> GhostResult<()> {
        let pending: Vec<PendingOutgoingMessage> =
            self.pending_outgoing_messages.drain(..).collect();
        for p in pending {
            let transport_request = transport::protocol::RequestToChild::SendMessage {
                uri: p.uri,
                payload: p.payload,
            };
            self.handle_transport_RequestToChild(p.span, transport_request, p.parent_request)?;
        }
        Ok(())
    }

    fn add_to_pending(
        &mut self,
        span: Span,
        uri: Lib3hUri,
        payload: Opaque,
        parent_request: GatewayToChildMessage,
    ) {
        self.pending_outgoing_messages.push(PendingOutgoingMessage {
            span,
            uri,
            payload,
            parent_request,
        });
    }

    /// Handle Transport request sent to use by our parent
    pub(crate) fn handle_transport_RequestToChild(
        &mut self,
        span: Span,
        transport_request: transport::protocol::RequestToChild,
        parent_request: GatewayToChildMessage,
    ) -> Lib3hResult<()> {
        match transport_request {
            transport::protocol::RequestToChild::Bind { spec: _ } => {
                // Forward to child transport
                let _ = self.inner_transport.as_mut().request(
                    span.child("handle_transport_RequestToChild"),
                    transport_request,
                    Box::new(|_me, response| {
                        let response = {
                            match response {
                                GhostCallbackData::Timeout => {
                                    parent_request
                                        .respond(Err(Lib3hError::new_other("timeout")))?;
                                    return Ok(());
                                }
                                GhostCallbackData::Response(response) => response,
                            }
                        };
                        // forward back to parent
                        parent_request.respond(Ok(GatewayRequestToChildResponse::Transport(
                            response.unwrap(),
                        )))?;
                        Ok(())
                    }),
                );
            }
            transport::protocol::RequestToChild::SendMessage { uri, payload } => {
                let payload_wrapped = payload.clone(); // not really wrapped

                // TODO - XXX - We need to wrap this so we know how / where
                //              to put this message (which gateway) on the
                //              remote side

                /*
                let to_agent_id = uri.path();
                trace!(
                    "try-send {:?} {} {} bytes",
                    self.identifier.id,
                    to_agent_id,
                    payload.len()
                );

                let request_id = nanoid::simple();
                // as a gateway, we need to wrap items going to our children
                let wrap_payload = P2pProtocol::DirectMessage(DirectMessageData {
                    space_address: self.identifier.id.clone(),
                    request_id: request_id.clone(),
                    to_agent_id: to_agent_id.into(),
                    from_agent_id: self.this_peer.peer_address.clone().into(),
                    content: payload.clone(),
                });

                // Serialize payload
                let mut payload_wrapped = Vec::new();
                wrap_payload
                    .serialize(&mut Serializer::new(&mut payload_wrapped))
                    .unwrap();
                let payload_wrapped = Opaque::from(payload_wrapped);
                */

                // uri is actually a dht peerKey
                // get actual uri from the inner dht before sending
                self.inner_dht.request(
                    span.child("transport::protocol::RequestToChild::SendMessage"),
                    DhtRequestToChild::RequestPeer(uri.clone()),
                    Box::new(move |me, response| {
                        match response {
                            GhostCallbackData::Response(Ok(
                                DhtRequestToChildResponse::RequestPeer(Some(peer_data)),
                            )) => {
                                me.send(
                                    span.follower("TODO send"),
                                    peer_data.peer_uri.clone(),
                                    payload_wrapped,
                                    Box::new(|response| {
                                        parent_request.respond(
                                            response
                                                .map_err(|transport_error| transport_error.into()),
                                        )
                                    }),
                                )?;
                            }
                            _ => {
                                debug!("Couldn't Send: {:?}", response);
                                me.add_to_pending(
                                    span.follower("retry_gateway_send"),
                                    uri,
                                    payload,
                                    parent_request,
                                );
                            }
                        };
                        Ok(())
                    }),
                )?;
            }
        }
        // Done
        Ok(())
    }

    /// handle RequestToChildResponse received from child Transport
    /// before forwarding it to our parent
    #[allow(dead_code)]
    pub(crate) fn handle_transport_RequestToChildResponse(
        &mut self,
        response: &transport::protocol::RequestToChildResponse,
    ) -> TransportResult<()> {
        match response {
            transport::protocol::RequestToChildResponse::Bind(_result_data) => {
                // no-op
            }
            transport::protocol::RequestToChildResponse::SendMessageSuccess => {
                // no-op
            }
        };
        Ok(())
    }

    /// Handle request received from child transport
    pub(crate) fn handle_transport_RequestToParent(
        &mut self,
        mut msg: transport::protocol::ToParentMessage,
    ) -> TransportResult<()> {
        debug!(
            "({}) Serving request from child transport: {:?}",
            self.identifier.nickname, msg
        );
        let span = msg.span().child("handle_transport_RequestToParent");
        match msg.take_message().expect("exists") {
            transport::protocol::RequestToParent::ErrorOccured { uri, error } => {
                // TODO
                error!(
                    "({}) Connection Error for {}: {}\n Closing connection.",
                    self.identifier.nickname, uri, error,
                );
            }
            transport::protocol::RequestToParent::IncomingConnection { uri } => {
                // TODO
                info!(
                    "({}) Incoming connection opened: {}",
                    self.identifier.nickname, uri
                );
                self.handle_incoming_connection(
                    span.child("transport::protocol::RequestToParent::IncomingConnection"),
                    uri.clone(),
                )?;
            }
            transport::protocol::RequestToParent::ReceivedData { uri, payload } => {
                // TODO
                trace!(
                    "{:?} Received message from: {} | size: {}",
                    self.identifier,
                    uri,
                    payload.len()
                );
                // trace!("Deserialize msg: {:?}", payload);
                if payload.len() == 0 {
                    debug!("Implement Ping!");
                } else {
                    let mut de = Deserializer::new(&payload[..]);
                    let maybe_p2p_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                        Deserialize::deserialize(&mut de);
                    if let Ok(p2p_msg) = maybe_p2p_msg {
                        if let P2pProtocol::PeerAddress(gateway_id, peer_address, timestamp) =
                            p2p_msg
                        {
                            debug!(
                                "Received PeerAddress: {} | {} ({})",
                                peer_address, gateway_id, self.identifier.nickname
                            );
                            if self.identifier.id == gateway_id.into() {
                                let peer = PeerData {
                                    peer_address,
                                    peer_uri: uri.clone(),
                                    timestamp,
                                };
                                // HACK
                                let _ = self.inner_dht.publish(
                                    span.follower(
                                        "transport::protocol::RequestToParent::ReceivedData",
                                    ),
                                    DhtRequestToChild::HoldPeer(peer),
                                );
                                // TODO #58
                                // TODO #150 - Should not call process manually
                                self.process().expect("HACK");
                            }
                        }
                    }
                }
            }
        };
        Ok(())
    }

    /// handle response we got from our parent
    #[allow(dead_code)]
    pub(crate) fn handle_transport_RequestToParentResponse(
        &mut self,
        _response: &transport::protocol::RequestToParentResponse,
    ) -> TransportResult<()> {
        // no-op
        Ok(())
    }
}
