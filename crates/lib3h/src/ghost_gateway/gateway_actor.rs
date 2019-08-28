use lib3h_ghost_actor::{
    prelude::*,
    ghost_channel::*,
};

impl<
    'gateway,
    D: Dht,
    RequestToParentContext,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    FakeError,
> GhostActor<
    RequestToParentContext,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    FakeError,
> for GhostGateway<'gateway, D> {

    fn as_any(&mut self) -> &mut dyn Any {
        &mut *self
    }

    fn take_parent_endpoint(&mut self) -> Option<TransportEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    /// Process
    fn /* priv */ process_concrete(&mut self) -> GhostResult<WorkWasDone> {

        // Check inbox from parent
        let endpoint_did_some_work = detach_run!(&mut self.endpoint_self, |endpoint_self| {
                endpoint_self.process(self.as_any())
            })?;
        let mut worked_for_parent = false;
        for mut msg in self.endpoint_self.as_mut().drain_messages() {
            match msg.take_message().expect("exists") {
                RequestToChild::Bind { spec: _ } => {
                    worked_for_parent = true;
                    self.bind(spec, msg)?;
                }
                RequestToChild::SendMessage {
                    address,
                    payload: _,
                } => {
                    worked_for_parent = true;
                    let mut id_ref_list = Vec::with_capacity(id_list.len());
                    for id in id_list {
                        id_ref_list.push(id.as_str());
                    }
                    let _id = self.send(&id_ref_list, payload, msg)?;
                }
            }
        }
        // Process child
        let child_did_some_work =detach_run!(&mut self.child_transport, |child_transport| {
            child_transport.process(self.as_any())
        })?;
        // Process internal dht
        let (dht_did_some_work, event_list) = self.inner_dht.process()?;
        // Done
        Ok(endpoint_did_some_work || dht_did_some_work || worked_for_parent || child_did_some_work)
    }
}

//--------------------------------------------------------------------------------------------------
// Private internals
//--------------------------------------------------------------------------------------------------

/// Private internals
impl<'gateway, D: Dht> GhostGateway<'gateway, D> {
    /// Forward Bind request to child Transport
    fn bind(
        &mut self,
        spec: &Url,
        parent_msg: &mut TransportMessage,
    ) -> TransportResult<()> {
        self.child_transport.as_mut().request(
            std::time::Duration::from_millis(2000), // FIXME magic number
            TransportContext::Bind { parent_msg },
            TransportRequestToChild::Bind { spec },
            // Should receive a response back from our message.
            // Forward it back to parent
            Box::new(|me, context, response| {
                let me = match me.downcast_mut::<GhostGateway>() {
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
                    msg.respond(Err(e));
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
        Ok()
    }

    /// id_list =
    ///   - Network : transportId
    ///   - space   : agentId
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn send(
        &mut self,
        dht_id_list: &[&ConnectionIdRef],
        payload: &[u8],
        parent_msg: &mut TransportMessage,
    ) -> TransportResult<()> {
        // get connectionId from the inner dht first
        let dht_uri_list = self.dht_address_to_uri_list(dht_id_list)?;
        trace!("({}).send() {:?} -> {:?} | {}", self.identifier, dht_id_list, dht_uri_list, payload.len());
        let ref_list: Vec<&str> = dht_uri_list;
        // Forward to the child Transport
        for address in ref_list {
            self.child_transport.as_mut().request(
                std::time::Duration::from_millis(2000), // FIXME magic number
                TransportContext::SendMessage { parent_msg },
                TransportRequestToChild::SendMessage { address, payload },
                // Might receive a response back from our message.
                // Send it back to parent
                Box::new(|me, context, response| {
                    let me = match me.downcast_mut::<GhostGateway>() {
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
                        msg.respond(Err(e));
                        return Ok(());
                    };
                    // Must be a SendMessage response
                    let send_response = match response {
                        TransportRequestToChildResponse::SendMessage(send_response) => send_response,
                        _ => panic!("received unexpected response type"),
                    };
                    println!("yay? {:?}", response);
                    // Act on response: forward to parent
                    parent_msg.respond(response);
                    // Done
                    Ok(())
                }),
            );
        }
        // Done
        Ok()
    }
}