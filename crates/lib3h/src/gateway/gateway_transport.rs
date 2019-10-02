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
use lib3h_protocol::{data_types::*, uri::Lib3hUri};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

/// Private internals
impl P2pGateway {
    /// Handle IncomingConnection event from child transport
    fn handle_incoming_connection(&mut self, span: Span, uri: Lib3hUri) -> TransportResult<()> {

        // TODO: This is prbably wrong in that a different level of URI should be being bubbled up.
        self.endpoint_self.publish(
            Span::fixme(),
            GatewayRequestToParent::Transport(
                transport::protocol::RequestToParent::IncomingConnection {
                    uri: uri.clone(),
                },
            ),
        )?;

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
                    me.send_with_full_low_uri(
                        SendWithFullLowUri {
                            span: span.follower("TODO send"),
                            full_low_uri: uri.clone(),
                            payload: buf.into(),
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

    fn priv_on_receive(&mut self, span: Span, uri: Lib3hUri, payload: Opaque) -> GhostResult<()> {
        let mut de = Deserializer::new(&payload[..]);
        let maybe_p2p_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
            Deserialize::deserialize(&mut de);

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
                    debug!("Implement Ping!");
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
