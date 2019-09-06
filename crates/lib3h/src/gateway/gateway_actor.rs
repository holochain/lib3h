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
        // process inbox from parent
        detach_run!(&mut self.endpoint_self, |es| es.process(&mut ()))?;
        // handle requests from parent
        for request in self.endpoint_self.as_mut().drain_messages() {
            //debug!("@MirrorDht@ serving request: {:?}", request);
            self.handle_request_from_parent(request)
                .expect("no ghost errors");
        }


        let mut worked_for_parent = false;
        for mut msg in self.endpoint_self.as_mut().unwrap().drain_messages() {
            match msg.take_message().expect("exists") {
                TransportRequestToChild::Bind { spec } => {
                    worked_for_parent = true;
                    // FIXME self.bind(&spec, Some(msg))?;
                }
                TransportRequestToChild::SendMessage { address, payload } => {
                    worked_for_parent = true;
                    // FIXME self.send(&address, &payload, Some(msg))?;
                }
            }
        }
        // Process child
        detach_run!(&mut self.child_transport, |child_transport| {
            child_transport.process(self.as_any())
        })?;
        // Act on child's requests
        for mut msg in self.child_transport.drain_messages() {
            let mut endpoint_self = std::mem::replace(&mut self.endpoint_self, None);
            match msg.take_message().expect("msg doesn't exist") {
                TransportRequestToParent::IncomingConnection { address } => {
                    // TODO
                    // bubble up to parent
                    endpoint_self.as_mut().expect("exists").publish(msg);
                }
                TransportRequestToParent::ReceivedData { address, payload } => {
                    // TODO
                    endpoint_self.as_mut().expect("exists").publish(msg);
                }
                TransportRequestToParent::ErrorOccured { address, error } => {
                    // TODO
                    endpoint_self.as_mut().expect("exists").publish(msg);
                }
            };
            std::mem::replace(&mut self.endpoint_self, endpoint_self);
        }
        // Process internal dht
        let (dht_did_some_work, event_list) = self.inner_dht.process().unwrap(); // fixme
                                                                                 // Handle DhtEvents
        if dht_did_some_work {
            for dht_evt in event_list {
                self.handle_netDhtEvent(dht_evt);
            }
        }
        // Done
        Ok(WorkWasDone::from(dht_did_some_work || worked_for_parent))
    }
}

//--------------------------------------------------------------------------------------------------
// Private internals
//--------------------------------------------------------------------------------------------------

impl P2pGateway {
    #[allow(irrefutable_let_patterns)]
    fn handle_request_from_parent(&mut self, mut request: DhtToChildMessage) -> Lib3hResult<()> {
        debug!("@P2pGateway@ serving request: {:?}", request);
        match request.take_message().expect("exists") {
            _ => (),
        }
        // Done
        Ok(())
    }
}
/// Private internals
impl<'gateway, D: Dht> GhostGateway<D> {
    /// Handle a DhtEvent sent to us by our network gateway
    fn handle_netDhtEvent(&mut self, cmd: DhtEvent) {
        debug!("{} << handle_netDhtEvent: {:?}", self.identifier, cmd);
        match cmd {
            DhtEvent::GossipTo(_data) => {
                // no-op
            }
            DhtEvent::GossipUnreliablyTo(_data) => {
                // no-op
            }
            DhtEvent::HoldPeerRequested(peer_data) => {
                // TODO #167 - hardcoded for MirrorDHT and thus should not appear here.
                // Connect to every peer we are requested to hold.
                info!(
                    "{} auto-connect to peer: {} ({})",
                    self.identifier, peer_data.peer_address, peer_data.peer_uri,
                );
                // Send phony SendMessage request so we connect to it
                self.child_transport
                    .publish(TransportRequestToChild::SendMessage {
                        address: peer_data.peer_uri,
                        payload: Vec::new(),
                    });
            }
            DhtEvent::PeerTimedOut(peer_address) => {
                // TODO
            }
            // No entries in Network DHT
            DhtEvent::HoldEntryRequested(_, _) => {
                unreachable!();
            }
            DhtEvent::FetchEntryResponse(_) => {
                unreachable!();
            }
            DhtEvent::EntryPruned(_) => {
                unreachable!();
            }
            DhtEvent::EntryDataRequested(_) => {
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
