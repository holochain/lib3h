#![allow(non_snake_case)]

use crate::{
    dht::dht_protocol::*,
    engine::p2p_protocol::P2pProtocol,
    error::*,
    gateway::{protocol::*, P2pGateway, PendingOutgoingMessage, SendCallback},
    transport::{self, error::TransportResult},
};
use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::data_types::*;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;

/// Private internals
impl P2pGateway {
    /// Handle IncomingConnection event from child transport
    fn handle_incoming_connection(&mut self, span: Span, uri: Url) -> TransportResult<()> {
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
                    // Send to other node our PeerAddress
                    let our_peer_address = P2pProtocol::PeerAddress(
                        me.identifier.id.to_owned().into(),
                        this_peer.peer_address,
                        this_peer.timestamp,
                    );
                    let mut buf = Vec::new();
                    our_peer_address
                        .serialize(&mut Serializer::new(&mut buf))
                        .unwrap();
                    trace!(
                        "({}) sending P2pProtocol::PeerAddress: {:?} to {:?}",
                        me.identifier.nickname,
                        our_peer_address,
                        uri,
                    );
                    me.inner_transport.request(
                        span,
                        transport::protocol::RequestToChild::SendMessage {
                            uri: uri.clone(),
                            payload: buf.into(),
                        },
                        Box::new(|_me, response| {
                            match response {
                                GhostCallbackData::Response(Err(e)) => error!(
                                    "Error exchanging peer info with new connection: {:?}",
                                    e,
                                ),
                                GhostCallbackData::Timeout => {
                                    error!("Timeout exchanging peer info with new connection")
                                }
                                _ => trace!("Successfully exchanged peer info with new connection"),
                            };
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

    /// uri =
    ///   - Network : transportId
    ///   - space   : agentId
    pub(crate) fn send(
        &mut self,
        span: Span,
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
        // Forward to the child Transport
        self.inner_transport.request(
            span.child("SendMessage"),
            transport::protocol::RequestToChild::SendMessage {
                uri: uri.clone(),
                payload: payload.clone(),
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
                        Err(
                            format!("Transport error while trying to send message: {:?}", error)
                                .into(),
                        )
                    }
                    // Timeout:
                    GhostCallbackData::Timeout => {
                        debug!("Gateway got timeout from transport. Adding message to pending");
                        Err("Ghost timeout error while trying to send message".into())
                    }
                }
            }),
        )
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
        uri: Url,
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
                let payload_wrapped = payload.clone(); // not actually wrapped

                // TODO - XXX - We need to wrap this so we know how / where
                //              to put this message (which gateway) on the
                //              remote side

                let to_agent_id = uri.path();

                let request_id = nanoid::simple();
                // as a gateway, we need to wrap items going to our children
                let wrap_payload = P2pProtocol::DirectMessage(DirectMessageData {
                    space_address: self.identifier.id.clone(),
                    request_id: request_id.clone(),
                    to_agent_id: to_agent_id.into(),
                    from_agent_id: self.this_peer.peer_address.clone().into(),
                    content: payload.clone(),
                });

                error!("try-send {:#?} {}", wrap_payload, uri);

                /*
                println!("{:?}", parent_request.backtrace());

                error!(
                    "try-send {:?} {} {:#?}",
                    self.identifier.id,
                    to_agent_id,
                    wrap_payload,
                );

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
                                        trace!("SENT!");
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
        let request = msg.take_message().expect("exists");
        match &request {
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
                debug!("Received message from: {} | size: {}", uri, payload.len());
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
        // Bubble up to parent
        self.endpoint_self.as_mut().publish(
            span.follower("bubble up to parent"),
            GatewayRequestToParent::Transport(request),
        )?;
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
