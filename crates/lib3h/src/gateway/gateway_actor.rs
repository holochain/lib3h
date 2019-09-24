use crate::{
    dht::dht_protocol::*,
    error::*,
    gateway::{protocol::*, P2pGateway},
};
use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::data_types::*;

impl
    GhostActor<
        GatewayRequestToParent,
        GatewayRequestToParentResponse,
        GatewayRequestToChild,
        GatewayRequestToChildResponse,
        Lib3hError,
    > for P2pGateway
{
    fn take_parent_endpoint(&mut self) -> Option<GatewayParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // process inbox from parent & handle requests
        detach_run!(&mut self.endpoint_self, |es| es.process(&mut ()))?;
        for request in self.endpoint_self.as_mut().drain_messages() {
            self.handle_RequestToChild(request)?;
        }

        // Process inbox from child transport & handle requests
        detach_run!(&mut self.inner_transport, |cte| { cte.process(self) })?;
        for request in self.inner_transport.drain_messages() {
            self.handle_transport_RequestToParent(request)?;
        }

        detach_run!(&mut self.message_encoding, |enc| { enc.process(self) })?;

        self.handle_transport_pending_outgoing_messages()?;

        // Update this_peer cache
        self.inner_dht.request(
            Span::fixme(),
            DhtRequestToChild::RequestThisPeer,
            Box::new(|mut me, response| {
                let response = {
                    match response {
                        GhostCallbackData::Timeout => panic!("timeout"),
                        GhostCallbackData::Response(response) => match response {
                            Err(e) => panic!("{:?}", e),
                            Ok(response) => response,
                        },
                    }
                };
                if let DhtRequestToChildResponse::RequestThisPeer(peer_response) = response {
                    trace!("Received RequestThisPeer response: {:?}", peer_response);
                    me.this_peer = peer_response;
                } else {
                    panic!("bad response to RequestThisPeer: {:?}", response);
                }
                Ok(())
            }),
        )?;

        // Process internal dht & handle requests
        detach_run!(self.inner_dht, |dht| { dht.process(self) })?;
        for request in self.inner_dht.drain_messages() {
            self.handle_dht_RequestToParent(request)?;
        }

        // Done
        Ok(WorkWasDone::from(true)) // FIXME
    }
}

/// Private internals
impl P2pGateway {
    fn handle_RequestToChild(&mut self, mut msg: GatewayToChildMessage) -> Lib3hResult<()> {
        debug!(
            "({}) Serving request from parent: {:?}",
            self.identifier.nickname, msg
        );
        // let parent_request = msg.clone();
        let span = msg.span().child("handle_RequestToChild");
        let request = msg.take_message().expect("exists");
        match request {
            GatewayRequestToChild::Transport(transport_request) => {
                // Forward to child transport
                self.handle_transport_RequestToChild(span, transport_request, msg)
            }
            GatewayRequestToChild::Dht(dht_request) => {
                // Forward to child dht
                self.handle_dht_RequestToChild(span, dht_request, msg)
            }
            GatewayRequestToChild::Bootstrap(data) => {
                self.send(
                    span,
                    // this will be fixed when we get Ping working
                    "".to_string().into(),
                    data.bootstrap_uri.clone(),
                    Opaque::new(),
                    Box::new(move |response| {
                        if response.is_ok() {
                            msg.respond(Ok(GatewayRequestToChildResponse::BootstrapSuccess))?;
                            Ok(())
                        } else {
                            msg.respond(Err(response.err().unwrap().into()))?;
                            Ok(())
                        }
                    }),
                )?;
                Ok(())
            }
            GatewayRequestToChild::SendAll(payload) => {
                self.inner_dht.request(
                    Span::fixme(),
                    DhtRequestToChild::RequestPeerList,
                    Box::new(move |me, response| {
                        match response {
                            GhostCallbackData::Timeout => panic!("Timeout on RequestPeerList"),
                            GhostCallbackData::Response(Err(error)) => {
                                panic!("Error on RequestPeerList: {:?}", error)
                            }
                            GhostCallbackData::Response(Ok(
                                DhtRequestToChildResponse::RequestPeerList(peer_list),
                            )) => {
                                for peer in peer_list {
                                    me.send(
                                        Span::fixme(),
                                        peer.peer_name.clone().into_agent_id(),
                                        peer.peer_location.clone(),
                                        payload.clone().into(),
                                        Box::new(move |response| {
                                            debug!(
                                                "P2pGateway::SendAll to {:?} response: {:?}",
                                                peer.peer_location, response
                                            );
                                            Ok(())
                                        }),
                                    )?;
                                }
                            }
                            _ => panic!("Whatever"),
                        }
                        Ok(())
                    }),
                )?;
                Ok(())
            }
        }
    }
}
