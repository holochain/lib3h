use crate::transport::{
    error::TransportError,
    memory_mock::memory_server::{self, *},
    protocol::*,
};
use detach::Detach;
use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use std::collections::HashSet;
use url::Url;

pub type UserData = GhostTransportMemory;

type GhostTransportMemoryEndpoint = GhostEndpoint<
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    TransportError,
>;

type GhostTransportMemoryEndpointContext = GhostContextEndpoint<
    UserData,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    TransportError,
>;

pub type GhostTransportMemoryEndpointContextParent = GhostContextEndpoint<
    Url,
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    TransportError,
>;

#[allow(dead_code)]
pub struct GhostTransportMemory {
    network: String,
    endpoint_parent: Option<GhostTransportMemoryEndpoint>,
    endpoint_self: Detach<GhostTransportMemoryEndpointContext>,
    /// My peer uri on the network layer (not None after a bind)
    maybe_my_address: Option<Url>,
    /// Addresses of connections to remotes
    connections: HashSet<Url>,
}

impl GhostTransportMemory {
    #[allow(dead_code)]
    pub fn new(network_name: &str) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        Self {
            network: network_name.to_string(),
            endpoint_parent: Some(endpoint_parent),
            endpoint_self: Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix("tmem_to_parent")
                    .build(),
            ),
            connections: HashSet::new(),
            maybe_my_address: None,
        }
    }
}

impl From<TransportError> for GhostError {
    fn from(e: TransportError) -> Self {
        format!("TransportError: {}", e).into()
    }
}

impl
    GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        TransportError,
    > for GhostTransportMemory
{
    // BOILERPLATE START----------------------------------

    fn take_parent_endpoint(&mut self) -> Option<GhostTransportMemoryEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    // BOILERPLATE END----------------------------------

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // make sure we have bound and get our address if so
        if let Some(my_addr) = &self.maybe_my_address {
            trace!("Processing for: {}", my_addr);

            // get our own server
            let (success, event_list) = {
                let mut verse = memory_server::get_memory_verse();
                match verse.get_server(&self.network, &my_addr) {
                    None => return Err(format!("No Memory server at this uri: {}", my_addr).into()),
                    Some(server) => server.process()?,
                }
            };
            if success {
                let mut to_connect_list: Vec<(Url)> = Vec::new();
                let mut non_connect_events = Vec::new();

                // process any connection events
                for event in event_list {
                    match event {
                        MemoryEvent::IncomingConnectionEstablished(in_cid) => {
                            to_connect_list.push(in_cid.clone());
                            self.endpoint_self.publish(
                                Span::fixme(),
                                RequestToParent::IncomingConnection {
                                    uri: in_cid.clone(),
                                },
                            )?;
                        }
                        _ => non_connect_events.push(event),
                    }
                }

                // Connect back to received connections if not already connected to them
                for remote_addr in to_connect_list {
                    debug!(
                        "(GhostTransportMemory)connecting {} <- {:?}",
                        remote_addr, my_addr
                    );

                    // if not already connected, request a connection
                    if self.connections.get(&remote_addr).is_none() {
                        // Get other node's server
                        let mut verse = memory_server::get_memory_verse();
                        match verse.get_server(&self.network, &remote_addr) {
                            Some(server) => {
                                server.request_connect(&my_addr)?;
                                self.connections.insert(remote_addr.clone());
                            }
                            None => {
                                return Err(format!(
                                    "No Memory server at this url address: {}",
                                    remote_addr
                                )
                                .into())
                            }
                        };
                    }
                }

                // process any other events
                for event in non_connect_events {
                    match event {
                        MemoryEvent::ReceivedData(from_addr, payload) => {
                            trace!("RecivedData--- from:{:?} payload:{:?}", from_addr, payload);
                            self.endpoint_self.publish(
                                Span::fixme(),
                                RequestToParent::ReceivedData {
                                    uri: from_addr,
                                    payload,
                                },
                            )?;
                        }
                        _ => panic!(format!("WHAT: {:?}", event)),
                    };
                }
            };
        };
        // process the self endpoint
        detach_run!(self.endpoint_self, |endpoint_self| endpoint_self
            .process(self))?;

        for mut msg in self.endpoint_self.drain_messages() {
            let _span = msg.span().child("process_concrete");
            match msg.take_message().expect("exists") {
                RequestToChild::Bind { spec: _url } => {
                    // get a new bound url from the memory server (we ignore the spec here)
                    let bound_url = {
                        let mut verse = memory_server::get_memory_verse();
                        verse.bind(&self.network)
                    };
                    self.maybe_my_address = Some(bound_url.clone());

                    // respond to our parent
                    msg.respond(Ok(RequestToChildResponse::Bind(BindResultData {
                        bound_url: bound_url,
                    })))?;
                }
                RequestToChild::SendMessage { uri, payload } => {
                    // make sure we have bound and get our address if so
                    //let my_addr = is_bound!(self, request_id, SendMessage);

                    // make sure we have bound and get our address if so
                    match &self.maybe_my_address {
                        None => {
                            msg.respond(Err(TransportError::new(
                                "Transport must be bound before sending".to_string(),
                            )))?;
                        }
                        Some(my_addr) => {
                            // get destinations server
                            let mut verse = memory_server::get_memory_verse();
                            match verse.get_server(&self.network, &uri) {
                                None => {
                                    msg.respond(Err(TransportError::new(format!(
                                        "No Memory server at this uri: {}",
                                        my_addr
                                    ))))?;
                                    continue;
                                }
                                Some(server) => {
                                    // if not already connected, request a connections
                                    if self.connections.get(&uri).is_none() {
                                        match server.request_connect(&my_addr) {
                                            Err(err) => {
                                                msg.respond(Err(err))?;
                                                continue;
                                            }
                                            Ok(()) => self.connections.insert(uri.clone()),
                                        };
                                    };
                                    trace!(
                                        "(GhostTransportMemory).SendMessage from {} to  {} | {:?}",
                                        my_addr,
                                        uri,
                                        payload
                                    );
                                    // Send it data from us
                                    server.post(&my_addr, &payload).unwrap();
                                }
                            }
                        }
                    };
                }
            }
        }
        Ok(true.into())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    //use protocol::RequestToChildResponse;
    use holochain_tracing::test_span;

    #[test]
    fn test_gmem_transport() {
        let mut transport1 = GhostTransportMemory::new("net1");
        let mut t1_endpoint: GhostTransportMemoryEndpointContextParent = transport1
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix("tmem_to_child1")
            .build::<Url>();

        let mut transport2 = GhostTransportMemory::new("net1");
        let mut t2_endpoint = transport2
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix("tmem_to_child2")
            .build::<Url>();

        // create two memory bindings so that we have addresses
        assert_eq!(transport1.maybe_my_address, None);
        assert_eq!(transport2.maybe_my_address, None);

        let mut bound_transport1_address = Url::parse("mem://addr_1").unwrap();
        t1_endpoint
            .request(
                test_span(""),
                RequestToChild::Bind {
                    spec: Url::parse("mem://_").unwrap(),
                },
                Box::new(|ud: &mut Url, r| {
                    match r {
                        GhostCallbackData::Response(Ok(RequestToChildResponse::Bind(
                            BindResultData { bound_url },
                        ))) => *ud = bound_url,
                        _ => assert!(false),
                    };
                    Ok(())
                }),
            )
            .unwrap();
        let mut bound_transport2_address = Url::parse("mem://addr_2").unwrap();
        t2_endpoint
            .request(
                test_span(""),
                RequestToChild::Bind {
                    spec: Url::parse("mem://_").unwrap(),
                },
                Box::new(|ud: &mut Url, r| {
                    match r {
                        GhostCallbackData::Response(Ok(RequestToChildResponse::Bind(
                            BindResultData { bound_url },
                        ))) => *ud = bound_url,
                        _ => assert!(false),
                    };
                    Ok(())
                }),
            )
            .unwrap();

        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut bound_transport1_address);

        transport2.process().unwrap();
        let _ = t2_endpoint.process(&mut bound_transport2_address);

        assert_eq!(
            transport1.maybe_my_address,
            Some(bound_transport1_address.clone())
        );
        assert_eq!(
            transport2.maybe_my_address,
            Some(bound_transport2_address.clone())
        );

        // now send a message from transport1 to transport2 over the bound addresses
        t1_endpoint
            .request(
                test_span(""),
                RequestToChild::SendMessage {
                    uri: Url::parse("mem://addr_2").unwrap(),
                    payload: b"test message".to_vec().into(),
                },
                Box::new(|_: &mut Url, r| {
                    // parent should see that the send request was OK
                    assert_eq!("Response(Ok(SendMessage))", &format!("{:?}", r));
                    Ok(())
                }),
            )
            .unwrap();

        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut bound_transport1_address);

        transport2.process().unwrap();
        let _ = t2_endpoint.process(&mut bound_transport2_address);

        let mut requests = t2_endpoint.drain_messages();
        assert_eq!(2, requests.len());
        assert_eq!(
            "Some(IncomingConnection { uri: \"mem://addr_1/\" })",
            format!("{:?}", requests[0].take_message())
        );
        assert_eq!(
            "Some(ReceivedData { uri: \"mem://addr_1/\", payload: \"test message\" })",
            format!("{:?}", requests[1].take_message())
        );
    }
}
