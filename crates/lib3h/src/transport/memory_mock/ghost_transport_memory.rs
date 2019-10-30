use crate::transport::{
    error::TransportError,
    memory_mock::memory_server::{self, *},
    protocol::{RequestToChildResponse::SendMessageSuccess, *},
};
use detach::Detach;
use holochain_tracing::Span;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{
    discovery::{
        error::{DiscoveryError, DiscoveryResult},
        Discovery,
    },
    types::*,
    uri::Lib3hUri,
};
use std::{collections::HashSet, sync::Arc, time::Instant};

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
    Lib3hUri,
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    TransportError,
>;

#[allow(dead_code)]
pub struct GhostTransportMemory {
    node_id: NodePubKey,
    network: Arc<GhostMutex<MemoryNet>>,
    endpoint_parent: Option<GhostTransportMemoryEndpoint>,
    endpoint_self: Detach<GhostTransportMemoryEndpointContext>,
    /// My peer uri on the network layer (not None after a bind)
    maybe_my_address: Option<Lib3hUri>,
    /// Addresses of connections to remotes
    connections: HashSet<Lib3hUri>,
    last_discover: Option<Instant>,
    discover_interval_ms: u128,
}

impl Discovery for GhostTransportMemory {
    fn advertise(&mut self) -> DiscoveryResult<()> {
        let uri = self
            .maybe_my_address
            .clone()
            .ok_or_else(|| DiscoveryError::new_other("must bind before advertising"))?;
        self.network.lock().advertise(uri, self.node_id.clone());
        Ok(())
    }
    fn discover(&mut self) -> DiscoveryResult<Vec<Lib3hUri>> {
        let nodes = self.network.lock().discover();
        Ok(nodes.into_iter().map(|(uri, _)| uri).collect())
    }
    fn release(&mut self) -> DiscoveryResult<()> {
        Ok(())
    }
    fn flush(&mut self) -> DiscoveryResult<()> {
        Ok(())
    }
}
const DEFAULT_DISCOVERY_INTERVAL_MS: u64 = 30000;

impl GhostTransportMemory {
    pub fn new(node_id: NodePubKey, network_name: &str) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let interval = DEFAULT_DISCOVERY_INTERVAL_MS;
        let start = Instant::now().checked_sub(std::time::Duration::from_millis(interval + 1));
        let network = {
            let mut verse = memory_server::get_memory_verse();
            verse.get_network(network_name)
        };
        Self {
            node_id,
            network,
            endpoint_parent: Some(endpoint_parent),
            endpoint_self: Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix("tmem_to_parent")
                    .build(),
            ),
            connections: HashSet::new(),
            maybe_my_address: None,
            last_discover: start,
            discover_interval_ms: u128::from(interval),
        }
    }

    fn try_discover(&mut self) {
        if let Some(t) = self.last_discover {
            if self.maybe_my_address.is_some()
                && t.elapsed().as_millis() > self.discover_interval_ms
            {
                if let Ok(nodes) = self.discover() {
                    if nodes.len() > 0 {
                        let my_addr = self.maybe_my_address.as_ref().expect("should have bound");
                        for found_uri in nodes {
                            if found_uri == *my_addr {
                                continue;
                            }
                            // if not already connected, request a connections
                            if self.connections.get(&found_uri).is_none() {
                                // Get other node's server
                                match self.network.lock().get_server(&found_uri) {
                                    Some(remote_server) => {
                                        let _result = remote_server.request_connect(&my_addr);
                                        self.connections.insert(found_uri.clone());
                                        trace!("Discovered {}, we are: {}", &found_uri, &my_addr);
                                        self.endpoint_self
                                            .publish(
                                                Span::fixme(),
                                                RequestToParent::IncomingConnection {
                                                    uri: found_uri.clone(),
                                                },
                                            )
                                            .expect("should be able to publish");
                                        self.last_discover = None;
                                    }
                                    None => return,
                                };
                            }
                        }
                    }
                }
                self.last_discover = Some(Instant::now());
            }
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
        // periodic discover
        self.try_discover();

        // make sure we have bound and get our address if so
        if let Some(my_addr) = &self.maybe_my_address {
            // get our own server
            let (success, event_list) = {
                match self.network.lock().get_server(&my_addr) {
                    None => return Err(format!("No Memory server at this uri: {}", my_addr).into()),
                    Some(server) => server.process()?,
                }
            };
            if success {
                let mut to_connect_list: Vec<(Lib3hUri)> = Vec::new();
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
                        match self.network.lock().get_server(&remote_addr) {
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
                            trace!(
                                "MemoryEvent::ReceivedData --- from:{:?} payload:{:?}",
                                from_addr,
                                payload
                            );
                            self.endpoint_self.publish(
                                Span::fixme(),
                                RequestToParent::ReceivedData {
                                    uri: from_addr,
                                    payload,
                                },
                            )?;
                        }
                        MemoryEvent::Unbind(url) => {
                            trace!("MemoryEvent::Unbind: {:?}", url);
                            self.endpoint_self
                                .publish(Span::fixme(), RequestToParent::Unbind(url))?;
                        }
                        MemoryEvent::ConnectionClosed(url) => {
                            trace!("MemoryEvent::ConnectionClosed: {:?}", url);
                            self.endpoint_self
                                .publish(Span::fixme(), RequestToParent::Disconnect(url))?;
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
            let mut span = msg.span().child("process_concrete");
            match msg.take_message().expect("exists") {
                RequestToChild::Bind { spec: _url } => {
                    // get a new bound url from the memory server (we ignore the spec here)
                    let bound_url = { self.network.lock().bind() };
                    self.maybe_my_address = Some(bound_url.clone());
                    self.advertise()
                        .map_err(|e| GhostError::from(e.to_string()))?;
                    span.event(format!("Bind {{{}}}", bound_url));
                    // respond to our parent
                    //msg.span = span.child("response");
                    msg.respond(Ok(RequestToChildResponse::Bind(BindResultData {
                        bound_url: bound_url,
                    })))?;
                }
                RequestToChild::SendMessage { uri, payload, .. } => {
                    trace!("mem send: {:?}", payload);
                    // make sure we have bound and get our address if so
                    //let my_addr = is_bound!(self, request_id, SendMessage);

                    span.event(format!("SendMessage to '{}'", uri));

                    // make sure we have bound and get our address if so
                    match &self.maybe_my_address {
                        None => {
                            msg.respond(Err(TransportError::new(
                                "Transport must be bound before sending".to_string(),
                            )))?;
                        }
                        Some(my_addr) => {
                            // get destinations server
                            match self.network.lock().get_server(&uri) {
                                None => {
                                    msg.respond(Err(TransportError::new(format!(
                                        "No Memory server at this uri: {}",
                                        uri
                                    ))))?;
                                    continue;
                                }
                                Some(server) => {
                                    // first check to see if we are sending to self
                                    if &uri == my_addr {
                                        // if so we can add the message directly to our own inbox
                                        trace!("Send-to-self: payload:{:?}", payload);
                                        self.endpoint_self.publish(
                                            span.child("send event RequestToParent::ReceivedData"),
                                            RequestToParent::ReceivedData { uri: uri, payload },
                                        )?;
                                    } else {
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
                                    msg.respond(Ok(SendMessageSuccess))?;
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

    fn make_test_transport(
        id: &str,
        net_name: &str,
    ) -> (
        GhostTransportMemory,
        GhostTransportMemoryEndpointContextParent,
    ) {
        let node_id = format!("fake_node_id{}", id).as_str().into();
        let req_id_prefix = format!("tmem_to_child{}", id);
        let mut transport = GhostTransportMemory::new(node_id, &net_name);
        let endpoint: GhostTransportMemoryEndpointContextParent = transport
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix(&req_id_prefix)
            .build::<Lib3hUri>();
        (transport, endpoint)
    }

    fn do_bind(endpoint: &mut GhostTransportMemoryEndpointContextParent) {
        endpoint
            .request(
                test_span(""),
                RequestToChild::Bind {
                    spec: Lib3hUri::with_memory(""),
                },
                Box::new(|ud: &mut Lib3hUri, r| {
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
    }

    #[test]
    fn test_gmem_transport_bind_and_discover() {
        let (mut transport1, mut t1_endpoint) = make_test_transport("1", "net1");
        let (mut transport2, mut t2_endpoint) = make_test_transport("2", "net1");

        // transport on a different network
        let (mut transport3, mut t3_endpoint) = make_test_transport("3", "net2");

        // create two memory bindings so that we have addresses
        assert_eq!(transport1.maybe_my_address, None);
        assert_eq!(transport2.maybe_my_address, None);

        let mut bound_transport1_address = Lib3hUri::with_undefined();
        do_bind(&mut t1_endpoint);
        let mut bound_transport2_address = Lib3hUri::with_undefined();
        do_bind(&mut t2_endpoint);
        let mut bound_transport3_address = Lib3hUri::with_undefined();
        do_bind(&mut t3_endpoint);

        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut bound_transport1_address);

        transport2.process().unwrap();
        let _ = t2_endpoint.process(&mut bound_transport2_address);

        transport3.process().unwrap();
        let _ = t3_endpoint.process(&mut bound_transport3_address);

        assert_eq!(
            transport1.maybe_my_address,
            Some(bound_transport1_address.clone())
        );
        assert_eq!(
            transport2.maybe_my_address,
            Some(bound_transport2_address.clone())
        );
        assert_eq!(
            transport3.maybe_my_address,
            Some(bound_transport3_address.clone())
        );

        // check that bindings were advertised
        let found = transport1.discover().unwrap();
        assert!(
            &format!("{}", found[0]) == "mem://addr_1/"
                || &format!("{}", found[0]) == "mem://addr_2/"
        );
        assert!(
            &format!("{}", found[1]) == "mem://addr_1/"
                || &format!("{}", found[1]) == "mem://addr_2/"
        );
        let found = transport3.discover().unwrap();
        assert_eq!(&format!("{}", found[0]), "mem://addr_1/"); // because of different network
    }

    #[test]
    fn test_gmem_transport_send() {
        let (mut transport1, mut t1_endpoint) = make_test_transport("1", "send_net1");
        let (mut transport2, mut t2_endpoint) = make_test_transport("2", "send_net1");
        let (mut transport3, mut t3_endpoint) = make_test_transport("3", "send_net2");
        let mut bound_transport1_address = Lib3hUri::with_undefined();
        do_bind(&mut t1_endpoint);
        let mut bound_transport2_address = Lib3hUri::with_undefined();
        do_bind(&mut t2_endpoint);
        let mut bound_transport3_address = Lib3hUri::with_undefined();
        do_bind(&mut t3_endpoint);
        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut bound_transport1_address);

        transport2.process().unwrap();
        let _ = t2_endpoint.process(&mut bound_transport2_address);

        // send a message from transport1 to transport2 over the bound addresses
        t1_endpoint
            .request(
                test_span(""),
                RequestToChild::create_send_message(
                    Lib3hUri::with_memory("addr_2"),
                    b"test message".to_vec().into(),
                ),
                Box::new(|_: &mut Lib3hUri, r| {
                    // parent should see that the send request was OK
                    assert_eq!("Response(Ok(SendMessageSuccess))", &format!("{:?}", r));
                    Ok(())
                }),
            )
            .unwrap();

        // and also a message to a non-existent address
        t1_endpoint
            .request(
                test_span(""),
                RequestToChild::create_send_message (
                    Lib3hUri::with_memory("addr_3"),
                    b"test message".to_vec().into(),
                ),
                Box::new(|_: &mut Lib3hUri, r| {
                    // parent should see that the send request was OK
                    assert_eq!("Response(Err(TransportError(Other(\"No Memory server at this uri: mem://addr_3/\"))))", &format!("{:?}", r));
                    Ok(())
                }),
            )
            .unwrap();

        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut bound_transport1_address);

        transport2.process().unwrap();
        let _ = t2_endpoint.process(&mut bound_transport2_address);

        transport3.process().unwrap();
        let _ = t3_endpoint.process(&mut bound_transport3_address);

        let requests = t1_endpoint.drain_messages();
        assert_eq!(1, requests.len());

        let mut requests = t2_endpoint.drain_messages();
        assert_eq!(3, requests.len());
        let msg = requests[0].take_message();
        // which url was discovered is non-deterministic
        assert!(
            "Some(IncomingConnection { uri: Lib3hUri(\"mem://addr_1/\") })" == format!("{:?}", msg)
                || "Some(IncomingConnection { uri: Lib3hUri(\"mem://addr_2/\") })"
                    == format!("{:?}", msg)
        );
        assert_eq!(
            "Some(IncomingConnection { uri: Lib3hUri(\"mem://addr_1/\") })",
            format!("{:?}", requests[1].take_message())
        );
        assert_eq!(
            "Some(ReceivedData { uri: Lib3hUri(\"mem://addr_1/\"), payload: \"test message\" })",
            format!("{:?}", requests[2].take_message())
        );
    }

    #[test]
    fn test_gmem_transport_send_to_self() {
        let (mut transport1, mut t1_endpoint) =
            make_test_transport("1", "gmem_transport_send_to_self");
        let mut bound_transport1_address = Lib3hUri::with_undefined();
        do_bind(&mut t1_endpoint);
        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut bound_transport1_address);

        // send a message from transport1 to self over the bound addresses
        t1_endpoint
            .request(
                test_span(""),
                RequestToChild::SendMessage {
                    uri: Lib3hUri::with_memory("addr_1"),
                    payload: b"test message".to_vec().into(),
                },
                Box::new(|_: &mut Lib3hUri, r| {
                    // parent should see that the send request was OK
                    assert_eq!("Response(Ok(SendMessageSuccess))", &format!("{:?}", r));
                    Ok(())
                }),
            )
            .unwrap();

        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut bound_transport1_address);

        let mut requests = t1_endpoint.drain_messages();
        assert_eq!(1, requests.len());
        let msg = requests[0].take_message();
        assert_eq!(
            "Some(ReceivedData { uri: Lib3hUri(\"mem://addr_1/\"), payload: \"test message\" })",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_gmem_disconnect() {
        let netname = "test_gmem_disconnect";
        let (mut transport1, mut t1_endpoint) = make_test_transport("1", netname);
        let (mut transport2, mut t2_endpoint) = make_test_transport("2", netname);
        let mut bound_transport1_address = Lib3hUri::with_undefined();
        do_bind(&mut t1_endpoint);
        let mut bound_transport2_address = Lib3hUri::with_undefined();
        do_bind(&mut t2_endpoint);
        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut bound_transport1_address);
        transport2.process().unwrap();
        let _ = t2_endpoint.process(&mut bound_transport2_address);

        // send a message from transport1 to transport2 over the bound addresses to establish the connection
        t1_endpoint
            .request(
                test_span(""),
                RequestToChild::create_send_message(
                    Lib3hUri::with_memory("addr_2"),
                    b"test message".to_vec().into(),
                ),
                Box::new(|_: &mut Lib3hUri, r| {
                    // parent should see that the send request was OK
                    assert_eq!("Response(Ok(SendMessageSuccess))", &format!("{:?}", r));
                    Ok(())
                }),
            )
            .unwrap();
        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut bound_transport1_address);
        transport2.process().unwrap();
        let _ = t2_endpoint.process(&mut bound_transport2_address);

        // now have transport1's connection drop
        let t1_uri = transport1.maybe_my_address.clone().unwrap();
        let t2_uri = transport2.maybe_my_address.clone().unwrap();
        {
            let network = {
                let mut verse = get_memory_verse();
                verse.get_network(netname)
            };
            let mut net = network.lock();
            let server = net
                .get_server(&t2_uri)
                .expect("there should be a server for to_uri");
            server.request_close(&t1_uri).expect("can disconnect");
        }

        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut bound_transport1_address);
        transport2.process().unwrap();
        let _ = t2_endpoint.process(&mut bound_transport2_address);

        let mut requests = t2_endpoint.drain_messages();
        assert_eq!(4, requests.len());
        assert_eq!(
            "Some(Disconnect(Lib3hUri(\"mem://addr_1/\")))",
            format!("{:?}", requests[3].take_message())
        );
    }
}
