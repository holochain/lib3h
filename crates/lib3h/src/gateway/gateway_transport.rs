#![allow(non_snake_case)]

use crate::{
    dht::dht_protocol::*,
    engine::p2p_protocol::P2pProtocol,
    error::*,
    gateway::{protocol::*, P2pGateway},
    transport::{self, error::TransportResult},
};
use holochain_tracing::HSpan;
use lib3h_ghost_actor::prelude::*;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;

/// Private internals
impl P2pGateway {
    /// Handle IncomingConnection event from child transport
    fn handle_incoming_connection(&mut self, span: HSpan, uri: Url) -> TransportResult<()> {
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
                        me.identifier.to_string(),
                        this_peer.peer_address,
                        this_peer.timestamp,
                    );
                    let mut buf = Vec::new();
                    our_peer_address
                        .serialize(&mut Serializer::new(&mut buf))
                        .unwrap();
                    trace!(
                        "({}) sending P2pProtocol::PeerAddress: {:?} to {:?}",
                        me.identifier,
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
                            panic!("TODO - why does this never get called?? {:?}", response);
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
        span: HSpan,
        uri: &Url,
        payload: &[u8],
        parent_msg: GatewayToChildMessage,
    ) -> GhostResult<()> {
        trace!("({}).send() {} | {}", self.identifier, uri, payload.len());
        // Forward to the child Transport
        self.inner_transport.request(
            span,
            transport::protocol::RequestToChild::SendMessage {
                uri: uri.clone(),
                payload: payload.to_vec().into(),
            },
            // Might receive a response back from our message.
            // Forward it back to parent
            Box::new(|_me, response| {
                // check for timeout
                let response = match response {
                    GhostCallbackData::Timeout => {
                        parent_msg.respond(Err(Lib3hError::new_other("timeout")))?;
                        return Ok(());
                    }
                    GhostCallbackData::Response(response) => response,
                };
                // Check if response is an error
                let response = match response {
                    Err(e) => {
                        parent_msg.respond(Err(Lib3hError::new(ErrorKind::TransportError(e))))?;
                        return Ok(());
                    }
                    Ok(response) => response,
                };
                // Must be a SendMessage response
                if let transport::protocol::RequestToChildResponse::SendMessageSuccess = response {
                    parent_msg.respond(Ok(GatewayRequestToChildResponse::Transport(
                        transport::protocol::RequestToChildResponse::SendMessageSuccess,
                    )))?;
                } else {
                    parent_msg.respond(Err(format!("bad response type: {:?}", response).into()))?;
                }
                Ok(())
            }),
        )
    }

    /// Handle Transport request sent to use by our parent
    pub(crate) fn handle_transport_RequestToChild(
        &mut self,
        span: HSpan,
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
                // uri is actually a dht peerKey
                // get actual uri from the inner dht before sending
                self.inner_dht.request(
                    span.child("transport::protocol::RequestToChild::SendMessage"),
                    DhtRequestToChild::RequestPeer(uri.to_string()),
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
                        if let DhtRequestToChildResponse::RequestPeer(maybe_peer_data) = response {
                            if let Some(peer_data) = maybe_peer_data {
                                me.send(
                                    span.follower("TODO send"),
                                    &peer_data.peer_uri,
                                    &payload,
                                    parent_request,
                                )
                                .unwrap(); // FIXME unwrap
                            } else {
                                parent_request.respond(Err(format!(
                                    "no peer found to send PeerData{{{:?}}} Message{{{:?}}}",
                                    maybe_peer_data, payload
                                )
                                .into()))?;
                            };
                        } else {
                            parent_request.respond(Err(format!(
                                "bad response to RequestPeer: {:?}",
                                response
                            )
                            .into()))?;
                        }
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
            self.identifier, msg
        );
        let span = msg.span().child("handle_transport_RequestToParent");
        let request = msg.take_message().expect("exists");
        match &request {
            transport::protocol::RequestToParent::ErrorOccured { uri, error } => {
                // TODO
                error!(
                    "({}) Connection Error for {}: {}\n Closing connection.",
                    self.identifier, uri, error,
                );
                // self.inner_transport.as_mut().close(id)?;
            }
            transport::protocol::RequestToParent::IncomingConnection { uri } => {
                // TODO
                info!("({}) Incoming connection opened: {}", self.identifier, uri);
                self.handle_incoming_connection(
                    span.child("transport::protocol::RequestToParent::IncomingConnection"),
                    uri.clone(),
                )?;
            }
            transport::protocol::RequestToParent::ReceivedData { uri, payload } => {
                // TODO
                debug!("Received message from: {} | size: {}", uri, payload.len());
                // trace!("Deserialize msg: {:?}", payload);
                let mut de = Deserializer::new(&payload[..]);
                let maybe_p2p_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                if let Ok(p2p_msg) = maybe_p2p_msg {
                    if let P2pProtocol::PeerAddress(gateway_id, peer_address, timestamp) = p2p_msg {
                        debug!(
                            "Received PeerAddress: {} | {} ({})",
                            peer_address, gateway_id, self.identifier
                        );
                        if self.identifier == gateway_id {
                            let peer = PeerData {
                                peer_address,
                                peer_uri: uri.clone(),
                                timestamp,
                            };
                            // HACK
                            let _ = self.inner_dht.publish(
                                span.follower("transport::protocol::RequestToParent::ReceivedData"),
                                DhtRequestToChild::HoldPeer(peer),
                            );
                            // TODO #58
                            // TODO #150 - Should not call process manually
                            self.process().expect("HACK");
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
