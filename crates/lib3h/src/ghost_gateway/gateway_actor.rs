use std::any::Any;
use url::Url;
use lib3h_ghost_actor::prelude::*;
use crate::{
    dht::{dht_protocol::*, dht_trait::Dht},
    transport::{
        protocol::*,
        error::TransportError,
    },
    ghost_gateway::GhostGateway,
};

impl<D: Dht> GhostActor<
    TransportRequestToParent,
    TransportRequestToParentResponse,
    TransportRequestToChild,
    TransportRequestToChildResponse,
    TransportError,
> for GhostGateway<D> {

    fn as_any(&mut self) -> &mut dyn Any {
        &mut *self
    }

    fn take_parent_endpoint(&mut self) -> Option<TransportEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn /* priv */ process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // Check inbox from parent
        let endpoint_did_some_work = detach_run!(&mut self.endpoint_self.unwrap(), |endpoint_self| {
            endpoint_self.unwrap().process(self.as_any())
        })?;
        let mut worked_for_parent = false;
        for mut msg in self.endpoint_self.as_mut().drain_messages() {
            match msg.take_message().expect("exists") {
                TransportRequestToChild::Bind { spec } => {
                    worked_for_parent = true;
                    self.bind(&spec, msg)?;
                }
                TransportRequestToChild::SendMessage {
                    address,
                    payload,
                } => {
                    worked_for_parent = true;
                    self.send(&address, &payload, msg)?;
                }
            }
        }
        // Process child
        let child_did_some_work = detach_run!(&mut self.child_transport, |child_transport| {
            child_transport.process(self.as_any())
        })?;
        // Act on child's requests
        if child_did_some_work {
            for mut msg in self.child_transport.drain_messages() {
                match msg.take_message().expect("msg doesn't exist") {
                    TransportRequestToParent::IncomingConnection { _address } => {
                        // TODO
                    }
                    TransportRequestToParent::ReceivedData { _address, _payload } => {
                        // TODO
                    }
                    TransportRequestToParent::ErrorOccured { _address, _error } => {
                        // TODO
                    }
                };
                // bubble up to parent
                let mut endpoint_self = std::mem::replace(&mut self.endpoint_self, None);
                endpoint_self.as_mut().expect("exists").publish(msg);
                std::mem::replace(&mut self.endpoint_self, endpoint_self);
            }
        }
        // Process internal dht
        let (dht_did_some_work, event_list) = self.inner_dht.process()?;
        // Handle DhtEvents
        if dht_did_some_work {
            for dht_evt in event_list {
                self.handle_netDhtEvent(evt);
            }
        }
        // Done
        Ok(WorkWasDone::from(endpoint_did_some_work || dht_did_some_work || worked_for_parent || child_did_some_work))
    }
}

//--------------------------------------------------------------------------------------------------
// Private internals
//--------------------------------------------------------------------------------------------------

/// Private internals
impl<'gateway, D: Dht> GhostGateway<D> {
    /// Handle a DhtEvent sent to us by our network gateway
    fn handle_netDhtEvent(&mut self, cmd: DhtEvent) {
        debug!("{} << handle_netDhtEvent: {:?}", self.name, cmd);
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
                    self.name, peer_data.peer_address, peer_data.peer_uri,
                );
                // Send phony SendMessage request so we connect to it
                self.network_gateway.as_mut().publish(
                    (),
                    TransportRequestToChild::SendMessage { address: peer_data.peer_uri, payload: Vec::new() },
                );
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
    fn bind(
        &mut self,
        spec: &Url,
        maybe_parent_msg: Option<TransportMessage>,
    ) -> GhostResult<()> {
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
                let parent_msg = {
                    if let TransportContext::Bind { parent_msg } = context {
                        parent_msg
                    } else {
                        panic!("received unexpected context type");
                    }
                };

                // check for timeout
                if let GhostCallbackData::Timeout = response {
                    parent_msg.respond(Err(TransportError::new("Timeout".into())));
                    return Ok(());
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
                    parent_msg.respond(Err(e));
                    return Ok(());
                };
                // Must be a SendMessage response
                let send_response =
                    if let TransportRequestToChildResponse::Bind(send_response) = response {
                    send_response
                } else {
                    panic!("received unexpected response type");
                }
                ;
                println!("yay? {:?}", response);
                // Act on response: forward to parent
                parent_msg.respond(response);
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
        let dht_uri_list = self.dht_address_to_uri_list([dht_id])?;
        let address = dht_uri_list[0];
        trace!("({}).send() {} -> {} | {}", self.identifier, dht_id, address, payload.len());
        // Forward to the child Transport
        self.child_transport.as_mut().request(
        std::time::Duration::from_millis(2000), // FIXME magic number
        TransportContext::SendMessage { maybe_parent_msg },
        TransportRequestToChild::SendMessage { address: address.clone(), payload: payload.to_vec() },
            // Might receive a response back from our message.
            // Send it back to parent
            Box::new(|me, context, response| {
                let me = match me.downcast_mut::<GhostGateway<D>>() {
                    None => panic!("received unexpected actor"),
                    Some(me) => me,
                };
                // Get parent's message from context
                let parent_msg = {
                    if let TransportContext::SendMessage { parent_msg } = context {
                        parent_msg
                    } else {
                        panic!("received unexpected context type");
                    }
                };
                // check for timeout
                if let GhostCallbackData::Timeout = response {
                    parent_msg.respond(Err("Timeout".into()));
                    return Ok(());
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
                    parent_msg.respond(Err(e));
                    return Ok(());
                };
                // Must be a SendMessage response
                let _ = match response {
                    TransportRequestToChildResponse::SendMessage => (),
                    _ => panic!("received unexpected response type"),
                };
                println!("yay? {:?}", response);
                // Act on response: forward to parent
                parent_msg.respond(response);
                // Done
                Ok(())
            }),
        );
        // Done
        Ok(())
    }
}