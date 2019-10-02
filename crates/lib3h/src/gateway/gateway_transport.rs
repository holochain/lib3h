#![allow(non_snake_case)]

use crate::{
    dht::dht_protocol::*,
    engine::p2p_protocol::P2pProtocol,
    error::*,
    gateway::{protocol::*, send_data_types::*, P2pGateway},
    message_encoding::encoding_protocol,
    transport::{self, error::TransportResult},
};
use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use lib3h_p2p_protocol::p2p::P2pMessage;
use lib3h_protocol::{data_types::*, uri::Lib3hUri};

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
                        GhostCallbackData::Timeout(bt) => panic!("timeout: {:?}", bt),
                        GhostCallbackData::Response(response) => match response {
                            Err(e) => panic!("{:?}", e),
                            Ok(response) => response,
                        },
                    }
                };
                if let DhtRequestToChildResponse::RequestThisPeer(this_peer) = response {
                    // once we have the peer info from the other side, bubble the incoming connection
                    // to the network layer
                    me.endpoint_self.publish(
                        Span::fixme(),
                        GatewayRequestToParent::Transport(
                            transport::protocol::RequestToParent::IncomingConnection {
                                uri: this_peer.peer_name.clone(),
                            },
                        ),
                    )?;

                    // Send to other node our PeerName
                    let our_peer_name = P2pProtocol::PeerName(
                        me.identifier.id.to_owned().into(),
                        this_peer.peer_name,
                        this_peer.timestamp,
                    );
                    trace!(
                        "({}) sending P2pProtocol::PeerName: {:?} to {:?}",
                        me.identifier.nickname,
                        our_peer_name,
                        uri,
                    );
                    let buf = our_peer_name.into_bytes().into();
                    me.send_with_full_low_uri(
                        SendWithFullLowUri {
                            span: span.follower("TODO send"),
                            full_low_uri: uri.clone(),
                            payload: buf,
                        },
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

    fn priv_decode_on_receive(
        &mut self,
        span: Span,
        uri: Lib3hUri,
        payload: Opaque,
    ) -> GhostResult<()> {
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
                            panic!("We should no longer ever be sending zero length messages");
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

    fn priv_on_receive(&mut self, span: Span, uri: Lib3hUri, payload: Opaque) -> GhostResult<()> {
        let maybe_p2p_msg = P2pProtocol::from_bytes(payload.into());
        match maybe_p2p_msg {
            Ok(P2pProtocol::PeerName(gateway_id, peer_name, timestamp)) => {
                if self.identifier.id != gateway_id.clone().into() {
                    panic!("BAD gateway {:?} != {:?}", self.identifier.id, gateway_id);
                }
                let peer = PeerData {
                    peer_name,
                    peer_location: uri.clone(),
                    timestamp,
                };
                debug!(
                    "{:?} Received PeerName: ({}) {} : {:?}",
                    self.this_peer, self.identifier.nickname, gateway_id, peer,
                );
                // HACK
                self.inner_dht.publish(
                    span.follower("transport::protocol::RequestToParent::ReceivedData"),
                    DhtRequestToChild::HoldPeer(peer),
                )?;
            }
            Ok(P2pProtocol::CapnProtoMessage(bytes)) => {
                match P2pMessage::from_bytes(bytes) {
                    Ok(P2pMessage::MsgPing(ping)) => {
                        debug!("got ping from {} {:?}", uri, ping);
                        let pong = P2pProtocol::CapnProtoMessage(
                            P2pMessage::create_pong(ping.ping_send_epoch_ms, None).into_bytes(),
                        )
                        .into_bytes()
                        .into();
                        self.send_with_full_low_uri(
                            SendWithFullLowUri {
                                span: Span::fixme(),
                                full_low_uri: uri,
                                payload: pong,
                            },
                            Box::new(move |response| {
                                // we don't need to follow up on a pong
                                // it can just be a fire-and-forget
                                trace!("sent pong {:?}", response);
                                Ok(())
                            }),
                        )?;
                    }
                    Ok(P2pMessage::MsgPong(pong)) => {
                        let now = crate::time::since_epoch_ms();
                        info!(
                            "got pong from {} indicating latency = {} ms",
                            uri,
                            now - pong.ping_send_epoch_ms,
                        );
                    }
                    _ => panic!("failed to decode P2pMessage"),
                }
            }
            Ok(msg) => {
                // TODO XXX - nope!
                // We should handle these cases, and pick the ones we want to
                // send up the chain, and which ones should be handled here.

                trace!("{:?} received {} {:?}", self.identifier, uri, msg,);

                self.endpoint_self.as_mut().publish(
                    span.follower("bubble up to parent"),
                    GatewayRequestToParent::Transport(
                        transport::protocol::RequestToParent::ReceivedData {
                            uri,
                            payload: msg.into_bytes().into(),
                        },
                    ),
                )?;
            }
            _ => {
                panic!("unexpected received data type {:?}", maybe_p2p_msg);
            }
        };
        Ok(())
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
                                GhostCallbackData::Timeout(bt) => {
                                    parent_request
                                        .respond(Err(format!("timeout: {:?}", bt).into()))?;
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
                debug!(
                    "gateway_transport: SendMessage, first resolving address {:?}",
                    uri.clone()
                );
                self.send_with_partial_high_uri(
                    SendWithPartialHighUri {
                        span: span.child("send_with_partial_high_uri"),
                        partial_high_uri: uri.clone(),
                        payload,
                    },
                    Box::new(|response| {
                        parent_request
                            .respond(response.map_err(|transport_error| transport_error.into()))
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
        trace!(
            "({}) Serving request from child transport: {:?}",
            self.identifier.nickname,
            msg
        );
        let span = msg.span().child("handle_transport_RequestToParent");
        let msg = msg.take_message().expect("exists");
        match &msg {
            transport::protocol::RequestToParent::ErrorOccured { uri: _, error: _ } => {
                // pass any errors back up the chain so network layer can handle them (i.e.)
                self.endpoint_self.publish(
                    Span::fixme(),
                    GatewayRequestToParent::Transport(msg.clone()),
                )?;
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
                    panic!("We should no longer ever be sending zero length messages");
                } else {
                    self.priv_decode_on_receive(span, uri.clone(), payload.clone())?;
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
