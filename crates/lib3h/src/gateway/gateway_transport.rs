#![allow(non_snake_case)]

use crate::{
    dht::dht_protocol::*,
    engine::p2p_protocol::P2pProtocol,
    error::*,
    gateway::{protocol::*, P2pGateway},
    transport::{
        self,
        error::{TransportError, TransportResult},
    },
};
use lib3h_ghost_actor::prelude::*;
use lib3h_tracing::Lib3hTrace;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug)]
pub enum TransportContext {
    Bind {
        maybe_parent_msg: Option<transport::protocol::ToChildMessage>,
    },
    SendMessage {
        maybe_parent_msg: Option<transport::protocol::ToChildMessage>,
    },
}

/// Private internals
impl P2pGateway {
    /// Get Uris from DHT peer_address'
    pub(crate) fn address_to_uri(&mut self, address_list: &[&str]) -> TransportResult<Vec<Url>> {
        let mut uri_list = Vec::with_capacity(address_list.len());
        for address in address_list {
            let maybe_peer = self.get_peer_sync(address);
            match maybe_peer {
                None => {
                    return Err(TransportError::new(format!(
                        "Unknown peerAddress: {}",
                        address
                    )));
                }
                Some(peer) => uri_list.push(peer.peer_uri),
            }
        }
        Ok(uri_list)
    }

    /// Handle IncomingConnection event from child transport
    fn handle_incoming_connection(&mut self, uri: Url) -> TransportResult<()> {
        // Send to other node our PeerAddress
        let this_peer = self.get_this_peer_sync().clone();
        let our_peer_address = P2pProtocol::PeerAddress(
            self.identifier.to_string(),
            this_peer.peer_address,
            this_peer.timestamp,
        );
        let mut buf = Vec::new();
        our_peer_address
            .serialize(&mut Serializer::new(&mut buf))
            .unwrap();
        trace!(
            "({}) sending P2pProtocol::PeerAddress: {:?} to {:?}",
            self.identifier,
            our_peer_address,
            uri,
        );
        let _res = self.send(&uri, &buf, None);
        Ok(())
    }

    /// id_list =
    ///   - Network : transportId
    ///   - space   : agentId
    fn send(
        &mut self,
        uri: &Url,
        payload: &[u8],
        maybe_parent_msg: Option<GatewayToChildMessage>,
    ) -> GhostResult<()> {
        trace!("({}).send() {} | {}", self.identifier, uri, payload.len());
        // Forward to the child Transport
        self.child_transport_endpoint.request(
            Lib3hTrace,
            transport::protocol::RequestToChild::SendMessage {
                uri: uri.clone(),
                payload: payload.to_vec().into(),
            },
            // Might receive a response back from our message.
            // Forward it back to parent
            Box::new(|_me, response| {
                // check for timeout
                if let GhostCallbackData::Timeout = response {
                    if let Some(parent_msg) = maybe_parent_msg {
                        parent_msg.respond(Err(Lib3hError::new_other("timeout")))?;
                        return Ok(());
                    }
                }
                // got response
                let response = {
                    if let GhostCallbackData::Response(response) = response {
                        response
                    } else {
                        unimplemented!();
                    }
                };
                // Check if response is an error
                if let Err(e) = response {
                    if let Some(parent_msg) = maybe_parent_msg {
                        parent_msg.respond(Err(Lib3hError::new(ErrorKind::TransportError(e))))?;
                    }
                    return Ok(());
                };
                let response = response.unwrap();
                // Must be a SendMessage response
                match response {
                    transport::protocol::RequestToChildResponse::SendMessage { payload: _ } => (),
                    _ => panic!("received unexpected response type"),
                };
                println!("yay? {:?}", response);
                // Act on response: forward to parent
                if let Some(parent_msg) = maybe_parent_msg {
                    parent_msg.respond(Ok(GatewayRequestToChildResponse::Transport(response)))?;
                }
                // Done
                Ok(())
            }),
        )
    }

    /// Handle Transport request sent to use by our parent
    pub(crate) fn handle_transport_RequestToChild(
        &mut self,
        transport_request: transport::protocol::RequestToChild,
        parent_request: GatewayToChildMessage,
    ) -> Lib3hResult<()> {
        match transport_request {
            transport::protocol::RequestToChild::Bind { spec: _ } => {
                // Forward to child transport
                let _ = self.child_transport_endpoint.as_mut().request(
                    Lib3hTrace,
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
                // uri is actually a dht peerAddress
                // get actual uri from the inner dht before sending
                let dht_uri_list = self.address_to_uri(&[&uri.to_string()])?;
                let dht_uri = &dht_uri_list[0];
                self.send(dht_uri, &payload, Some(parent_request))?;
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
            transport::protocol::RequestToChildResponse::SendMessage { payload: _ } => {
                // no-op
            }
        };
        Ok(())
    }

    /// Handle request received from child transport
    pub(crate) fn handle_transport_RequestToParent(
        &mut self,
        mut msg: transport::protocol::ToParentMessage,
    ) {
        debug!(
            "({}) Serving request from child transport: {:?}",
            self.identifier, msg
        );
        let request = msg.take_message().expect("msg doesn't exist");
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
                let _res = self.handle_incoming_connection(uri.clone());
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
                            let _ = self.inner_dht.publish(DhtRequestToChild::HoldPeer(peer));
                            // TODO #58
                            // TODO #150 - Should not call process manually
                            self.process().expect("HACK");
                        }
                    }
                }
            }
        };
        // Bubble up to parent
        let _res = self
            .endpoint_self
            .as_mut()
            .publish(GatewayRequestToParent::Transport(request));
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
