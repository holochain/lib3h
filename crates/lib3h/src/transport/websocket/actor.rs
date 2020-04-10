use crate::transport::{
    error::{TransportError, TransportResult},
    protocol::*,
    websocket::{
        streams::{ConnectionStatus, StreamEvent, StreamManager},
        tls::TlsConfig,
    },
};
use detach::Detach;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{
    data_types::Opaque,
    discovery::{
        error::{DiscoveryError, DiscoveryResult, ErrorKind as DiscoveryErrorKind},
        Discovery,
    },
    types::*,
    uri::Lib3hUri,
};
use std::{collections::HashSet, time::Instant};

// Use mDNS for bootstrapping
use crate::new_root_span;
use holochain_persistence_api::hash::HashString;
use lib3h_mdns::{MulticastDns, MulticastDnsBuilder};

pub type Message =
    GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>;

pub struct GhostTransportWebsocket {
    #[allow(dead_code)]
    node_id: NodePubKey,
    network_id_address: NetworkHash,
    endpoint_parent: Option<GhostTransportWebsocketEndpoint>,
    endpoint_self: Detach<GhostTransportWebsocketEndpointContext>,
    streams: StreamManager<std::net::TcpStream>,
    bound_url: Option<Lib3hUri>,
    pending: Vec<Message>,

    // mDNS specific variables
    mdns: Option<MulticastDns>,
    connections: HashSet<Lib3hUri>,
    last_discover: Option<Instant>,
    discover_interval_ms: u128,
}

// Here we just need to use mDNS, but use it only once, with advertise probably, and that's all.
impl Discovery for GhostTransportWebsocket {
    fn advertise(&mut self) -> DiscoveryResult<()> {
        // Lazily instantiate mDNS
        if self.mdns.is_none() {
            let uri: url::Url = self
                .bound_url
                .clone()
                .ok_or_else(|| DiscoveryError::new_other("Must bind URL before advertising."))?
                .into();

            let network_id_str: String = HashString::from(self.network_id_address.clone()).into();

            let mut mdns = MulticastDnsBuilder::new()
                .own_record(&network_id_str, &[&uri.clone().into_string()])
                .build()?;
            mdns.insert_record(&network_id_str, &[&uri.into_string()]);

            self.mdns = Some(mdns);
        }

        match &mut self.mdns {
            Some(mdns) => mdns.advertise(),
            None => {
                error!("Fail to advertise: No mDNS instance found.");
                Err(DiscoveryError::new(DiscoveryErrorKind::Other(
                    "No mDNS instance found during Advertising step.".to_string(),
                )))
            }
        }
    }

    fn discover(&mut self) -> DiscoveryResult<Vec<Lib3hUri>> {
        match &mut self.mdns {
            Some(mdns) => mdns.discover(),
            // None => Ok(Vec::new()),
            None => {
                self.advertise()?;
                if let Some(mdns) = &mut self.mdns {
                    mdns.discover()
                } else {
                    error!("Fail to advertise: No mDNS instance found.");
                    Err(DiscoveryError::new(DiscoveryErrorKind::Other(
                        "No mDNS instance found during Discovering step.".to_string(),
                    )))
                }
            }
        }
    }

    fn release(&mut self) -> DiscoveryResult<()> {
        match &mut self.mdns {
            Some(mdns) => mdns.release(),
            None => {
                warn!("mDNS Discovery: Fail to release.");
                Ok(())
            }
        }
    }

    fn flush(&mut self) -> DiscoveryResult<()> {
        Ok(())
    }
}

impl Drop for GhostTransportWebsocket {
    fn drop(&mut self) {
        self.streams
            .close_all()
            .unwrap_or_else(|e| error!("Error closing streams: {:?}", e));
    }
}

impl GhostTransportWebsocket {
    pub fn new(
        node_id: NodePubKey,
        tls_config: TlsConfig,
        network_id_address: NetworkHash,
    ) -> GhostTransportWebsocket {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        GhostTransportWebsocket {
            node_id,
            network_id_address,
            endpoint_parent: Some(endpoint_parent),
            endpoint_self: Detach::new(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix("twss_to_parent")
                    .build(),
            ),
            streams: StreamManager::with_std_tcp_stream(tls_config),
            bound_url: None,
            pending: Vec::new(),
            mdns: None,
            connections: HashSet::new(),
            last_discover: None,
            discover_interval_ms: 1_000,
        }
    }

    pub fn bound_url(&self) -> Option<Lib3hUri> {
        self.bound_url.clone()
    }

    /// Actually sends the message via an existing stream.
    /// Assumptions:
    /// * msg is a RequestToChild::SendMessage
    /// * a connection to the messages target URI is already there and ready
    /// If we encounter an error while sending, it will return an Err with the
    /// message object so it can be put back into the pending list tried again later.
    fn handle_send_message(&mut self, mut msg: Message) -> Result<(), Message> {
        match msg
            .take_message()
            .expect("GhostMessage must have inner RequestToChild")
        {
            RequestToChild::SendMessage { uri, payload, .. } => {
                match self.bound_url.clone() {
                    None => {
                        let _ = msg.respond(Err(TransportError::new(
                            "Transport must be bound before sending".to_string(),
                        )));
                        Ok(())
                    }
                    Some(my_addr) => {
                        trace!(
                            "(GhostTransportWebsocket).SendMessage from {} to  {} | {:?}",
                            my_addr,
                            uri,
                            payload
                        );
                        let bytes = payload.as_bytes();
                        // Send it data from us
                        if let Err(error) = self.streams.send(&uri, &bytes) {
                            trace!("Error during StreamManager::send: {:?}", error);
                            // In case of an error we reconstruct the GhostMessage and return
                            // it as error so the calling context can put it back into the pending
                            // list to try again later.
                            let payload = Opaque::from(bytes);
                            msg.put_message(RequestToChild::create_send_message(uri, payload));
                            Err(msg)
                        } else {
                            let _ = msg.respond(Ok(RequestToChildResponse::SendMessageSuccess));
                            Ok(())
                        }
                    }
                }
            }
            _ => panic!(
                "GhostTransportWebsocket::handle_send_message called with non-SendMessage message"
            ),
        }
    }

    fn process_actor_inbox(&mut self) -> TransportResult<()> {
        for mut msg in self.endpoint_self.drain_messages() {
            match msg.take_message().expect("exist") {
                RequestToChild::Bind { spec: url } => {
                    let maybe_bound_url = self.streams.bind(&url);
                    msg.respond(maybe_bound_url.clone().map(|url| {
                        RequestToChildResponse::Bind(BindResultData {
                            bound_url: url.into(),
                        })
                    }))?;

                    if let Ok(url) = maybe_bound_url {
                        trace!("Websocket binding to: {}", url);
                        self.bound_url = Some(url.clone().into());
                        self.advertise()
                            .map_err(|e| TransportError::from(e.to_string()))?;
                    }
                }
                RequestToChild::SendMessage { uri, payload, .. } => {
                    // make sure we have bound and got our address
                    if self.bound_url.is_none() {
                        msg.respond(Err(TransportError::new(
                            "Transport must be bound before sending".to_string(),
                        )))?;
                    } else {
                        // Trying to find established connection for URI:
                        match self.streams.connection_status(&uri) {
                            ConnectionStatus::None => {
                                // If there is none, try to connect:
                                trace!(
                                    "No open connection to {} found when sending data. Trying to connect...",
                                    uri,
                                );
                                match self.streams.connect(&uri) {
                                    Ok(()) => {
                                        trace!("New connection to {} initialized", uri.to_string())
                                    }
                                    Err(error) => {
                                        trace!(
                                            "Could not connect to {}! Transport error: {:?}",
                                            uri.to_string(),
                                            error
                                        );
                                    }
                                }

                                // And save message for later:
                                msg.put_message(RequestToChild::create_send_message(uri, payload));
                                self.pending.push(msg);
                            }
                            ConnectionStatus::Initializing => {
                                trace!("Send tried while initializing");
                                // If the connection is there but not ready yet, save message for later
                                msg.put_message(RequestToChild::create_send_message(uri, payload));
                                self.pending.push(msg);
                            }
                            ConnectionStatus::Ready => {
                                trace!("Send via previously established connection");
                                msg.put_message(RequestToChild::create_send_message(uri, payload));
                                if let Err(msg) = self.handle_send_message(msg) {
                                    trace!(
                                        "Error while sending message, putting it in pending list"
                                    );
                                    self.pending.push(msg);
                                }
                            }
                        }
                    };
                }
            }
        }
        Ok(())
    }

    fn process_stream_events(&mut self, stream_events: Vec<StreamEvent>) -> TransportResult<()> {
        for event in stream_events {
            let span = new_root_span("StreamEvent");
            match event {
                StreamEvent::ErrorOccured(uri, error) => {
                    warn!(
                        "Error in GhostTransportWebsocket stream connection to {:?}: {:?}",
                        uri, error
                    );
                    let response = match error.kind() {
                        crate::transport::error::ErrorKind::Other(err) => {
                            if err == "Protocol(\"Connection reset without closing handshake\")" {
                                trace!("rude disconnect from {}", uri);
                                RequestToParent::Disconnect(uri.into())
                            } else {
                                RequestToParent::ErrorOccured {
                                    uri: uri.into(),
                                    error,
                                }
                            }
                        }
                        _ => RequestToParent::ErrorOccured {
                            uri: uri.into(),
                            error,
                        },
                    };
                    self.endpoint_self
                        .publish(span.child("publish ErrorOccured"), response)?;
                }
                StreamEvent::ConnectResult(uri_connnected, _) => {
                    trace!("StreamEvent::ConnectResult: {:?}", uri_connnected);
                }
                StreamEvent::IncomingConnectionEstablished(uri) => {
                    trace!("StreamEvent::IncomingConnectionEstablished: {:?}", uri);
                    self.endpoint_self.publish(
                        span.child("send event RequestToParent::IncomingConnection"),
                        RequestToParent::IncomingConnection { uri: uri.into() },
                    )?;
                }
                StreamEvent::ReceivedData(uri, payload) => {
                    trace!(
                        "StreamEvent::ReceivedData: {:?}",
                        String::from_utf8(payload.clone())
                    );
                    self.endpoint_self.publish(
                        span.child("send event RequestToParent::ReceivedData"),
                        RequestToParent::ReceivedData {
                            uri: uri.into(),
                            payload: Opaque::from(payload),
                        },
                    )?;
                }
                StreamEvent::ConnectionClosed(uri) => {
                    trace!("StreamEvent::ConnectionClosed: {}", uri);
                    self.endpoint_self.publish(
                        span.child("send event RequestToParent::ConnectionClosed"),
                        RequestToParent::Disconnect(uri.into()),
                    )?;
                }
            }
        }
        Ok(())
    }

    fn process_pending_messages(&mut self) -> TransportResult<()> {
        let mut temp = Vec::new();
        while let Some(mut msg) = self.pending.pop() {
            trace!("Processing pending message...");
            let inner_msg = msg.take_message().expect("exists");
            if let RequestToChild::SendMessage { uri, payload, .. } = inner_msg {
                if self.streams.connection_status(&uri) == ConnectionStatus::Ready {
                    trace!("Sending pending message to: {:?}", uri);
                    msg.put_message(RequestToChild::create_send_message(uri, payload));
                    if let Err(msg) = self.handle_send_message(msg) {
                        trace!("Error while sending message, putting it back in pending list");
                        temp.push(msg);
                    }
                } else {
                    msg.put_message(RequestToChild::create_send_message(uri, payload));
                    temp.push(msg);
                }
            } else {
                panic!("Found a non-SendMessage message in GhostTransportWebsocket::pending!");
            }
        }
        self.pending = temp;
        Ok(())
    }

    /// Try to discover peers on the network using mDNS.
    fn try_discover(&mut self) {
        self.last_discover = match self.last_discover {
            None => {
                // Here we call a mDNS discovery at least once, which will take care of calling an
                // advertise for us if it's the fist time the mDNS actor is invoked
                match self.discover() {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Fail to discover during 'try_discover': {:?}", e);
                    }
                }
                Some(Instant::now())
            }
            Some(last_discover) => {
                // Let's check if it's time to discover some peers, but only if we already
                // did some url binding
                if self.bound_url.is_some()
                    && last_discover.elapsed().as_millis() > self.discover_interval_ms
                {
                    if self.discover_interval_ms < 30_000 {
                        // TODO:  <2019-10-09, dymayday> //
                        // This is a futur proof solution, right now we only need to discover one other
                        // peer, and the rest of the discovery, finding other nodes, is made by through
                        // gossip mechanism.

                        // // Increase the time between two peer discovery to avoid unnecessary burden on
                        // // the network
                        // self.discover_interval_ms += 1_000;

                        // This is the temporary solution then: we set the interval to the max
                        // possible value, so this code block is called only once
                        self.discover_interval_ms = ::std::u128::MAX;
                    }

                    // Do the peer discovery
                    if let Ok(machines) = self.discover() {
                        if !machines.is_empty() {
                            let my_addr = self.bound_url.as_ref().expect(
                                "'bound_url' already checked earlier, so it should not be None.",
                            );

                            for found_uri in machines {
                                if found_uri == *my_addr {
                                    // Ignoring our own address
                                    continue;
                                } else {
                                    // if not already connected, request a connections
                                    if self.connections.get(&found_uri).is_none() {
                                        // The proper way to connect is by using 'streams'
                                        match self.streams.connect(&found_uri) {
                                            Ok(()) => {
                                                self.connections.insert(found_uri.clone());
                                                trace!(
                                                    "New connection to {} initialized using mDNS",
                                                    &found_uri
                                                );
                                            }
                                            Err(error) => {
                                                trace!(
                                                    "Could not connect to {}! Transport error: {:?}",
                                                    found_uri.to_string(),
                                                    error
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Some(Instant::now())
                } else {
                    if self.bound_url.is_none() {
                        trace!("URL not bound yet.");
                    }
                    self.last_discover
                }
            }
        }
    }
}

pub type UserData = GhostTransportWebsocket;

pub type GhostTransportWebsocketEndpoint = GhostEndpoint<
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    TransportError,
>;

pub type GhostTransportWebsocketEndpointContext = GhostContextEndpoint<
    UserData,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    TransportError,
>;

pub type GhostTransportWebsocketEndpointContextParent<T> = GhostContextEndpoint<
    T,
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    TransportError,
>;

impl
    GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        TransportError,
    > for GhostTransportWebsocket
{
    // BOILERPLATE START----------------------------------

    fn take_parent_endpoint(&mut self) -> Option<GhostTransportWebsocketEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    // BOILERPLATE END----------------------------------

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // Check if the needed url binding happened before calling 'try_discover'
        if self.bound_url.is_some() {
            // Periodic peer discovery
            self.try_discover();
        }

        // process the self endpoint
        detach_run!(self.endpoint_self, |endpoint_self| endpoint_self
            .process(self))?;

        self.process_actor_inbox()?;

        // make sure we have bound and get our address if so
        let _my_addr = match &self.bound_url {
            Some(my_addr) => my_addr.clone(),
            None => return Ok(false.into()),
        };

        let (did_work, stream_events) = self.streams.process()?;
        self.process_stream_events(stream_events)?;
        self.process_pending_messages()?;

        Ok(did_work.into())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        tests::enable_logging_for_test, transport::websocket::tls::TlsConfig, wait_for_bind_result,
    };
    use lib3h_ghost_actor::{wait1_for_callback, wait_for_message, wait_until_no_work};
    use std::net::TcpListener;
    use url::Url;

    fn port_is_available(port: u16) -> bool {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn get_available_port(start: u16) -> Option<u16> {
        (start..65535).find(|port| port_is_available(*port))
    }

    #[test]
    fn test_websocket_transport_send_direct_msg() {
        let network_id_address: NetworkHash = "wss-bootstapping-network-id1.holo.host".into();

        let node_id_1 = "fake_node_id1".into();
        let mut transport1 = GhostTransportWebsocket::new(
            node_id_1,
            TlsConfig::Unencrypted,
            network_id_address.clone(),
        );

        let mut t1_endpoint: GhostTransportWebsocketEndpointContextParent<Option<String>> =
            transport1
                .take_parent_endpoint()
                .expect("exists")
                .as_context_endpoint_builder()
                .request_id_prefix("twss_to_child1")
                .build::<Option<String>>();

        let node_id_2 = "fake_node_id2".into();
        let mut transport2 = GhostTransportWebsocket::new(
            node_id_2,
            TlsConfig::Unencrypted,
            network_id_address.clone(),
        );

        let mut t2_endpoint = transport2
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix("twss_to_child2")
            .build::<Option<String>>();

        // create two memory bindings so that we have addresses
        assert_eq!(transport1.bound_url, None);
        assert_eq!(transport2.bound_url, None);

        let init_transport1_address: Lib3hUri =
            lib3h_protocol::uri::Builder::with_raw_url("wss://127.0.0.1/")
                .unwrap()
                .with_port(1024)
                .build();

        let (_is_match, expected_transport1_address) =
            wait_for_bind_result!(transport1, t1_endpoint, init_transport1_address);

        let port2 = get_available_port(expected_transport1_address.port().unwrap_or_else(|| 0))
            .expect("Must be able to find free port");

        let init_transport2_address: Lib3hUri = expected_transport1_address.with_port(port2);

        let (_is_match, expected_transport2_address) =
            wait_for_bind_result!(transport2, t2_endpoint, init_transport2_address);
        assert_eq!(
            transport1.bound_url(),
            Some(expected_transport1_address.clone())
        );
        assert_eq!(
            transport2.bound_url(),
            Some(expected_transport2_address.clone())
        );

        // now send a message from transport1 to transport2 over the bound addresses
        t1_endpoint
            .request(
                holochain_tracing::test_span(),
                RequestToChild::create_send_message(
                    expected_transport2_address.clone(),
                    b"test message".to_vec().into(),
                ),
                Box::new(|_: &mut _, r| {
                    // parent should see that the send request was OK
                    assert_eq!("Response(Ok(SendMessageSuccess))", &format!("{:?}", r));
                    Ok(())
                }),
            )
            .unwrap();

        wait_for_message!(
            vec![&mut transport1, &mut transport2],
            t2_endpoint,
            None,
            "ReceivedData \\{ uri: Lib3hUri\\(\"wss://127\\.0\\.0\\.1:\\d+/\"\\), payload: \"test message\" \\}"
        );
    }

    #[test]
    fn test_websocket_transport_reconnect() {
        enable_logging_for_test(true);

        let network_id_address: NetworkHash = "wss-bootstapping-network-id.holo.host".into();

        let node_id_1 = "fake_node_id1".into();
        let mut transport1 = GhostTransportWebsocket::new(
            node_id_1,
            TlsConfig::Unencrypted,
            network_id_address.clone(),
        );

        let mut t1_endpoint: GhostTransportWebsocketEndpointContextParent<_> = transport1
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix("twss_to_child1")
            .build::<Option<String>>();

        let port1 = get_available_port(2025).expect("Must be able to find free port");

        let init_transport1_address: Lib3hUri =
            lib3h_protocol::uri::Builder::with_raw_url("wss://127.0.0.1/")
                .unwrap()
                .with_port(port1)
                .build();

        let (_is_match, expected_transport1_address) =
            wait_for_bind_result!(transport1, t1_endpoint, init_transport1_address);

        assert_eq!(
            transport1.bound_url(),
            Some(expected_transport1_address.clone())
        );

        let port2 = get_available_port(2026).expect("Must be able to find free port");

        for index in 1..10 {
            wait_until_no_work!(transport1);
            {
                let node_id_2 = "fake_node_id2".into();
                let mut transport2 = GhostTransportWebsocket::new(
                    node_id_2,
                    TlsConfig::Unencrypted,
                    network_id_address.clone(),
                );

                let mut t2_endpoint = transport2
                    .take_parent_endpoint()
                    .expect("exists")
                    .as_context_endpoint_builder()
                    .request_id_prefix("twss_to_child2")
                    .build::<Option<String>>();

                let init_transport2_address: Lib3hUri =
                    Url::parse(&format!("wss://127.0.0.1:{}", port2))
                        .unwrap()
                        .into();
                let (_is_match, expected_transport2_address) =
                    wait_for_bind_result!(transport2, t2_endpoint, init_transport2_address);

                assert_eq!(
                    transport2.bound_url(),
                    Some(expected_transport2_address.clone())
                );

                let wait_callback_options =
                    crate::lib3h_ghost_actor::ghost_test_harness::ProcessingOptions {
                        // TODO set this to true
                        should_abort: false,
                        max_iters: 1,
                        ..Default::default()
                    };

                wait1_for_callback!(
                    transport1,
                    t1_endpoint,
                    RequestToChild::create_send_message(
                        expected_transport2_address.clone(),
                        b"test message".to_vec().into(),
                    ),
                    "Response(Ok(SendMessageSuccess))",
                    wait_callback_options
                );

                wait_for_message!(
                    vec![&mut transport1, &mut transport2],
                    t2_endpoint,
                    None,
                    "ReceivedData \\{ uri: Lib3hUri\\(\"wss://127\\.0\\.0\\.1:\\d+/\"\\), payload: \"test message\" \\}"
                );
            }

            println!("Try {} successful!", index);
        }
    }

    #[test]
    fn should_invoke_drop() {
        // TODO
    }

    #[test]
    fn try_discover_test() {
        // TODO
    }

    /// Check if we manage to discover nodes using WebSocket for bootstapping using mDNS.
    #[test]
    fn mdns_wss_bootstrapping_test() {
        let network_id_address: NetworkHash =
            format!("wss-bootstapping-network-id-{}.holo.host", nanoid::simple())
                .as_str()
                .into();

        let node_id_1 = "fake_node_id1".into();
        let mut transport1 = GhostTransportWebsocket::new(
            node_id_1,
            TlsConfig::Unencrypted,
            network_id_address.clone(),
        );
        let mut t1_endpoint: GhostTransportWebsocketEndpointContextParent<_> = transport1
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix("twss_to_child1")
            .build::<()>();

        // Here we should get an error because we indeed did not bind yet, so it's the expected
        // behavior
        assert_eq!(
            "Err(DiscoveryError(Other(\"Must bind URL before advertising.\")))",
            format!("{:?}", transport1.discover())
        );

        let node_id_2 = "fake_node_id2".into();
        let mut transport2 = GhostTransportWebsocket::new(
            node_id_2,
            TlsConfig::Unencrypted,
            network_id_address.clone(),
        );
        let mut t2_endpoint = transport2
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix("twss_to_child2")
            .build::<()>();

        // create two memory bindings so that we have addresses
        assert_eq!(transport1.bound_url, None);
        assert_eq!(transport2.bound_url, None);

        let port1 = get_available_port(3125).expect("Must be able to find free port");
        let expected_transport1_address: Lib3hUri =
            Url::parse(&format!("wss://127.0.0.1:{}", port1))
                .unwrap()
                .into();
        t1_endpoint
            .request(
                holochain_tracing::test_span(),
                RequestToChild::Bind {
                    spec: expected_transport1_address.clone(),
                },
                Box::new(move |_: &mut (), r| {
                    // parent should see the bind event
                    assert_eq!(
                        format!(
                            "Response(Ok(Bind(BindResultData {{ bound_url: Lib3hUri(\"wss://127.0.0.1:{}/\") }})))",
                            port1.clone(),
                        ),
                        format!("{:?}", r)
                    );
                    Ok(())
                }),
            )
            .unwrap();

        let port2 = get_available_port(3126).expect("Must be able to find free port");
        let expected_transport2_address: Lib3hUri =
            Url::parse(&format!("wss://127.0.0.1:{}", port2))
                .unwrap()
                .into();
        t2_endpoint
            .request(
                holochain_tracing::test_span(),
                RequestToChild::Bind {
                    spec: expected_transport2_address.clone(),
                },
                Box::new(move |_: &mut (), r| {
                    // parent should see the bind event
                    assert_eq!(
                        &format!(
                            "Response(Ok(Bind(BindResultData {{ bound_url: Lib3hUri(\"wss://127.0.0.1:{}/\") }})))",
                            port2.clone(),
                        ),
                        &format!("{:?}", r)
                    );
                    Ok(())
                }),
            )
            .unwrap();

        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut ());

        assert_eq!(
            transport1.bound_url(),
            Some(expected_transport1_address.clone())
        );

        // Advertise happens during bind so before processing transport2
        // transport1 will only know about itself on the call to discover
        let urls = transport1
            .discover()
            .expect("Fail to discover nodes using WSS transport1.");
        assert_eq!(vec![expected_transport1_address.clone()], urls);

        transport2.process().unwrap();
        let _ = t2_endpoint.process(&mut ());
        assert_eq!(
            transport2.bound_url(),
            Some(expected_transport2_address.clone())
        );

        // After transport2 bind processes, then discover on transport1 should show
        // transport2s bound address
        let urls = transport1
            .discover()
            .expect("Fail to discover nodes using WSS transport1.");

        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&expected_transport2_address));
    }
}
