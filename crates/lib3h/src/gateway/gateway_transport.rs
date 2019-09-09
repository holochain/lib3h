#![allow(non_snake_case)]

use crate::{
    dht::dht_protocol::*,
    engine::p2p_protocol::P2pProtocol,
    gateway::{Gateway, P2pGateway},
    transport::error::{TransportError, TransportResult},
};
use lib3h_protocol::DidWork;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use url::Url;


/// Private internals
impl P2pGateway {
//    /// TODO: return a higher-level uri instead
//    fn get_uri(&self, id: &ConnectionIdRef) -> Option<Url> {
//        self.inner_transport.as_ref().get_uri(id)
//        //let maybe_peer_data = self.inner_dht.get_peer(id);
//        //maybe_peer_data.map(|pd| pd.peer_address)
//    }

    /// Get Uris from DHT peer_address'
    pub(crate) fn address_to_uri(
        &mut self,
        address_list: &[&str],
    ) -> TransportResult<Vec<Url>> {
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
    fn handle_incoming_connection(&mut self, uri: &Url) -> TransportResult<()> {
        // Send to other node our PeerAddress
        let this_peer = self.get_this_peer_sync().clone();
        let our_peer_address = P2pProtocol::PeerAddress(
            self.identifier().to_string(),
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
            id,
        );
        return self.send(uri, &buf, None);
    }

    /// id_list =
    ///   - Network : transportId
    ///   - space   : agentId
    fn send(
        &mut self,
        uri: &Url,
        payload: &[u8],
        maybe_parent_msg: Option<TransportMessage>,
    ) -> GhostResult<()> {
        trace!(
            "({}).send() {} | {}",
            self.identifier,
            uri,
            payload.len()
        );
        // Forward to the child Transport
        self.child_transport.as_mut().request(
            TransportContext::SendMessage { maybe_parent_msg },
            TransportRequestToChild::SendMessage {
                uri: uri.clone(),
                payload: payload.to_vec(),
            },
            // Might receive a response back from our message.
            // Forward it back to parent
            Box::new(|_me, context, response| {
//                let me = match me.downcast_mut::<GhostGateway<D>>() {
//                    None => panic!("received unexpected actor"),
//                    Some(me) => me,
//                };
                // Get parent's message from context
                let maybe_parent_msg = {
                    if let GatewayContext::SendMessage { maybe_parent_msg } = context {
                        maybe_parent_msg
                    } else {
                        panic!("received unexpected context type");
                    }
                };
                // check for timeout
                if let GhostCallbackData::Timeout = response {
                    if let Some(parent_msg) = maybe_parent_msg {
                        parent_msg.respond(Err(TransportError::new("Timeout".into())));
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
                        parent_msg.respond(Err(e));
                    }
                    return Ok(());
                };
                let response = response.unwrap();
                // Must be a SendMessage response
                let response = match response {
                    transport::protocol::RequestToChildResponse::SendMessage { payload: _ } => (),
                    _ => panic!("received unexpected response type"),
                };
                println!("yay? {:?}", response);
                // Act on response: forward to parent
                if let Some(parent_msg) = maybe_parent_msg {
                    parent_msg.respond(GatewayRequestToChildResponse::Transport(response));
                }
                // Done
                Ok(())
            }),
        );
        // Done
        Ok(())
    }
}

/// Private internals
impl P2pGateway {
    /// Handle Transport request sent to use by our parent
    fn handle_transport_RequestToChild(
        &mut self,
        mut transport_request: protocol::transport::RequestToChild,
        mut parent_request: GatewayRequestToChild,
    ) -> Lib3hResult<()> {
        match transport_request.clone() {
            transport::protocol::RequestToChild::Bind { spec: _ } => {
                // Forward to child transport
                let _ = self.child_transport_endpoint.request(
                    GatewayContext::Transport { parent_request },
                    transport_request,
                    Box::new(|_me, context, response| {
                        let msg = {
                            match context {
                                GatewayContext::Transport { parent_request } => parent_request,
                                _ => {
                                    return Err(
                                        format!("wanted GatewayContext::Transport, got {:?}", context).into()
                                    )
                                }
                            }
                        };
                        let response = {
                            match response {
                                GhostCallbackData::Timeout => {
                                    msg.respond(Err("timeout".into()))?;
                                    return Ok(());
                                }
                                GhostCallbackData::Response(response) => response,
                            }
                        };
                        // forward back to parent
                        msg.respond(GatewayRequestToChildResponse::Transport(response))?;
                        Ok(())
                    }),
                );
            },
            transport::protocol::RequestToChild::SendMessage { uri, payload } => {
                // uri is actually a dht peerAddress
                // get actual uri from the inner dht before sending
                let dht_uri_list = self.address_to_uri(&[uri])?;
                let dht_uri = dht_uri_list[0];
                self.send(dht_uri, payload, Some(parent_request));
            },
        }
        // Done
        Ok(())
    }

    /// handle RequestToChildResponse received from child Transport
    /// before forwarding it to our parent
    pub(crate) fn handle_transport_RequestToChildResponse(
        &mut self,
        response: &protocol::transport::RequestToParentResponse,
    ) -> TransportResult<()> {
        match response {
            Bind(result_data) => {
                // no-op
            },
            SendMessage { payload: _ }=> {
                // no-op
            },
        };
        Ok(())
    }

    /// Handle request received from child transport
    fn handle_transport_RequestToParent(
        &mut self,
        mut request: protocol::transport::ToParentMessage,
    ) -> Lib3hResult<()> {
        debug!(
            "({}) Serving request from child transport: {:?}",
            self.identifier, request
        );
        match msg.take_message().expect("msg doesn't exist") {
            transport::protocol::RequestToParent::ErrorOccured { uri, error } => {
                // TODO
                error!(
                    "({}) Connection Error for {}: {}\n Closing connection.",
                    self.identifier, uri, error,
                );
                // self.inner_transport.as_mut().close(id)?;
            },
            transport::protocol::RequestToParent::IncomingConnection { uri } => {
                // TODO
                info!("({}) Incoming connection opened: {}", self.identifier, uri);
                self.handle_incoming_connection(uri)?;
            },
            transport::protocol::RequestToParent::ReceivedData { uri, payload } => {
                // TODO
                debug!("Received message from: {} | size: {}", uri, payload.len());
                // trace!("Deserialize msg: {:?}", payload);
                let mut de = Deserializer::new(&payload[..]);
                let maybe_p2p_msg: Result<P2pProtocol, rmp_serde::decode::Error> =
                    Deserialize::deserialize(&mut de);
                if let Ok(p2p_msg) = maybe_p2p_msg {
                    if let P2pProtocol::PeerAddress(gateway_id, peer_address, peer_timestamp) =
                    p2p_msg
                    {
                        debug!(
                            "Received PeerAddress: {} | {} ({})",
                            peer_address, gateway_id, self.identifier
                        );
                        let peer_uri = self
                            .inner_transport
                            .as_mut()
                            .get_uri(connection_id)
                            .expect("FIXME"); // TODO #58
                        debug!("peer_uri of: {} = {}", connection_id, peer_uri);
                        if self.identifier == gateway_id {
                            let peer = PeerData {
                                peer_address: peer_address.clone(),
                                peer_uri,
                                timestamp: peer_timestamp,
                            };
                            // HACK
                            self.hold_peer(peer);
                            // TODO #58
                            // TODO #150 - Should not call process manually
                            self.process().expect("HACK");
                        }
                    }
                }
            },
        };
        // Bubble up to parent
        self.endpoint_self.as_mut().expect("exists").publish(GatewayRequestToParent::Transport(msg));
    }

    /// handle response we got from our parent
    pub(crate) fn handle_transport_RequestToParentResponse(
        &mut self,
        response: &protocol::transport::RequestToParentResponse,
    ) -> TransportResult<()> {
        // no-op
        Ok(())
    }
}
