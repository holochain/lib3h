use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    gateway::protocol::*,
    transport::{error::TransportError, protocol::*},
};
use lib3h_ghost_actor::prelude::*;
use url::Url;

impl GatewayActor for P2pGateway
{
    fn take_parent_endpoint(&mut self) -> Option<GatewayParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // process inbox from parent & handle requests
        detach_run!(&mut self.endpoint_self, |es| es.process(&mut ()))?;
        for request in self.endpoint_self.as_mut().drain_messages() {
            self.handle_request_from_parent(request)
                .expect("no ghost errors");
        }

        // Process inbox from child transport & handle requests
        detach_run!(&mut self.child_transport_endpoint, |child_transport_endpoint| {
            child_transport_endpoint.process(self.as_any())
        })?;
        for request in self.child_transport_endpoint.as_mut().drain_messages() {
            self.handle_request_from_child_transport(request)
                .expect("no ghost errors");
        }

        // Process internal dht & handle requests
        let _res = self.inner_dht.process(&mut self.user_data);
        for request in self.inner_dht.drain_messages() {
            self.handle_request_from_child_dht(request)
                .expect("no ghost errors");
        }

        // Done
        Ok(WorkWasDone::from(true)) // FIXME
    }
}
//--------------------------------------------------------------------------------------------------
// Private internals
//--------------------------------------------------------------------------------------------------

impl P2pGateway {
    fn handle_request_from_parent(&mut self, mut request: GatewayToChildMessage) -> Lib3hResult<()> {
        debug!("({}) Serving request from parent: {:?}", self.identifier, request);
        match request.take_message().expect("exists") {
            Transport(transport_request) => {
                // Forward to child ???
                let _ = self.child_transport_endpoint.request(/* FIXME */);

//                match transport_request {
//                    transport::protocol::RequestToChild::Bind { spec } => {
//                        // FIXME
//                        self.bind(&spec, Some(msg))?;
//                    }
//                    transport::protocol::RequestToChild::SendMessage { address, payload } => {
//                        // FIXME
//                        self.send(&address, &payload, Some(msg))?;
//                    }
//                }
            },
            Dht(dht_request) => {
                let _ = self.inner_dht.request(/* FIXME */);
            }
            _ => (), // FIXME
        }
        // Done
        Ok(())
    }

    fn handle_request_from_child_transport(&mut self, mut request: TransportToParentMessage) -> Lib3hResult<()> {
        debug!("({}) Serving request from child transport: {:?}", self.identifier, request);
        match msg.take_message().expect("msg doesn't exist") {
                    transport::protocol::RequestToParent::IncomingConnection { address } => {
                        // TODO
                        // bubble up to parent
                        self.endpoint_self.as_mut().expect("exists").publish(msg);
                    }
                    transport::protocol::RequestToParent::ReceivedData { address, payload } => {
                        // TODO
                        self.endpoint_self.as_mut().expect("exists").publish(msg);
                    }
                    transport::protocol::RequestToParent::ErrorOccured { address, error } => {
                        // TODO
                        self.endpoint_self.as_mut().expect("exists").publish(msg);
                    }
                }
    }
}

/// Private internals
impl<'gateway, D: Dht> GhostGateway<D> {
    /// Handle a DhtEvent sent to us by our network gateway
    fn handle_request_from_child_dht(&mut self, mut request: DhtToParentMessage) {
        debug!("({}) Serving request from child dht: {:?}", self.identifier, request);
        match request.take_message().expect("exists") {
            DhtRequestToParent::GossipTo(_data) => {
                // no-op
            }
            DhtRequestToParent::GossipUnreliablyTo(_data) => {
                // no-op
            }
            DhtRequestToParent::HoldPeerRequested(peer_data) => {
                // TODO #167 - hardcoded for MirrorDHT and thus should not appear here.
                // Connect to every peer we are requested to hold.
                info!(
                    "{} auto-connect to peer: {} ({})",
                    self.identifier, peer_data.peer_address, peer_data.peer_uri,
                );
                // Send phony SendMessage request so we connect to it
                self.child_transport_endpoint
                    .publish(TransportRequestToChild::SendMessage {
                        address: peer_data.peer_uri,
                        payload: Vec::new(),
                    });
            }
            DhtRequestToParent::PeerTimedOut(peer_address) => {
                // TODO
            }
            // No entries in Network DHT
            DhtRequestToParent::HoldEntryRequested(_, _) => {
                unreachable!();
            }
            DhtRequestToParent::EntryPruned(_) => {
                unreachable!();
            }
            DhtRequestToParent::RequestEntry(_) => {
                unreachable!();
            }
        }
    }

    /// Forward Bind request to child Transport
    fn bind(&mut self, spec: &Url, maybe_parent_msg: Option<TransportMessage>) -> GhostResult<()> {
        self.child_transport.as_mut().request(
            std::time::Duration::from_millis(2000), // FIXME magic number
            TransportContext::Bind { maybe_parent_msg },
            TransportRequestToChild::Bind { spec: spec.clone() },
            // Should receive a response back from our message.
            // Forward it back to parent
            Box::new(|me, context, response| {
                let me = match me.downcast_mut::<GhostGateway<D>>() {
                    None => panic!("received unexpected actor"),
                    Some(me) => me,
                };
                // Get parent's message from context
                let maybe_parent_msg = {
                    if let TransportContext::Bind { maybe_parent_msg } = context {
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
                // Must be a Bind response
                let bind_response_data =
                    if let TransportRequestToChildResponse::Bind(bind_response_data) =
                        response.unwrap()
                    {
                        bind_response_data
                    } else {
                        panic!("received unexpected response type");
                    };
                println!("yay? {:?}", bind_response_data);
                // Act on response: forward to parent
                if let Some(parent_msg) = maybe_parent_msg {
                    //parent_msg.respond(response);
                    parent_msg.respond(TransportRequestToChildResponse::Bind(bind_response_data));
                }
                // Done
                Ok(())
            }),
        );
        // Done
        Ok(())
    }

    /// id_list =
    ///   - Network : transportId
    ///   - space   : agentId
    fn send(
        &mut self,
        dht_id: &Url,
        payload: &[u8],
        maybe_parent_msg: Option<TransportMessage>,
    ) -> GhostResult<()> {
        // get connectionId from the inner dht first
        let address = dht_id;
        trace!(
            "({}).send() {} | {}",
            self.identifier,
            address,
            payload.len()
        );
        // Forward to the child Transport
        self.child_transport.as_mut().request(
            std::time::Duration::from_millis(2000), // FIXME magic number
            TransportContext::SendMessage { maybe_parent_msg },
            TransportRequestToChild::SendMessage {
                address: address.clone(),
                payload: payload.to_vec(),
            },
            // Might receive a response back from our message.
            // Send it back to parent
            Box::new(|me, context, response| {
                let me = match me.downcast_mut::<GhostGateway<D>>() {
                    None => panic!("received unexpected actor"),
                    Some(me) => me,
                };
                // Get parent's message from context
                let maybe_parent_msg = {
                    if let TransportContext::SendMessage { maybe_parent_msg } = context {
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
                let _ = match response {
                    TransportRequestToChildResponse::SendMessage => (),
                    _ => panic!("received unexpected response type"),
                };
                println!("yay? {:?}", response);
                // Act on response: forward to parent
                if let Some(parent_msg) = maybe_parent_msg {
                    parent_msg.respond(response);
                }
                // Done
                Ok(())
            }),
        );
        // Done
        Ok(())
    }
}
