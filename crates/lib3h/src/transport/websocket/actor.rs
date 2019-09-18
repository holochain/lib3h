use crate::transport::{
    error::{TransportError, TransportResult},
    protocol::*,
    websocket::{
        streams::{ConnectionStatus, StreamEvent, StreamManager},
        tls::TlsConfig,
    },
};
use detach::Detach;
use lib3h_discovery::{error::DiscoveryResult, Discovery};
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::{data_types::Opaque, Address};
use lib3h_tracing::Lib3hSpan;
use url::Url;

pub type Message =
    GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>;

pub struct GhostTransportWebsocket {
    #[allow(dead_code)]
    machine_id: Address,
    endpoint_parent: Option<GhostTransportWebsocketEndpoint>,
    endpoint_self: Detach<GhostTransportWebsocketEndpointContext>,
    streams: StreamManager<std::net::TcpStream>,
    bound_url: Option<Url>,
    pending: Vec<Message>,
}

impl Discovery for GhostTransportWebsocket {
    fn advertise(&mut self) -> DiscoveryResult<()> {
        Ok(())
    }
    fn discover(&mut self) -> DiscoveryResult<Vec<Url>> {
        let nodes = Vec::new();
        Ok(nodes)
    }
    fn release(&mut self) -> DiscoveryResult<()> {
        Ok(())
    }
    fn flush(&mut self) -> DiscoveryResult<()> {
        Ok(())
    }
}

impl GhostTransportWebsocket {
    pub fn new(machine_id: Address, tls_config: TlsConfig) -> GhostTransportWebsocket {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        GhostTransportWebsocket {
            machine_id,
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
        }
    }

    pub fn bound_url(&self) -> Option<Url> {
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
            RequestToChild::SendMessage { uri, payload } => {
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
                            msg.put_message(RequestToChild::SendMessage { uri, payload });
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
                        RequestToChildResponse::Bind(BindResultData { bound_url: url })
                    }))?;

                    if let Ok(url) = maybe_bound_url {
                        self.bound_url = Some(url.clone());
                    }
                }
                RequestToChild::SendMessage { uri, payload } => {
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
                                msg.put_message(RequestToChild::SendMessage { uri, payload });
                                self.pending.push(msg);
                            }
                            ConnectionStatus::Initializing => {
                                trace!("Send tried while initializing");
                                // If the connection is there but not ready yet, save message for later
                                msg.put_message(RequestToChild::SendMessage { uri, payload });
                                self.pending.push(msg);
                            }
                            ConnectionStatus::Ready => {
                                trace!("Send via previously established connection");
                                msg.put_message(RequestToChild::SendMessage { uri, payload });
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
            match event {
                StreamEvent::ErrorOccured(uri, error) => {
                    warn!(
                        "Error in GhostWebsocketTransport stream connection to {:?}: {:?}",
                        uri, error
                    );
                    self.endpoint_self.publish(
                        Lib3hSpan::fixme(),
                        RequestToParent::ErrorOccured { uri, error },
                    )?;
                }
                StreamEvent::ConnectResult(uri_connnected, _) => {
                    trace!("StreamEvent::ConnectResult: {:?}", uri_connnected);
                }
                StreamEvent::IncomingConnectionEstablished(uri) => {
                    trace!("StreamEvent::IncomingConnectionEstablished: {:?}", uri);
                    self.endpoint_self.publish(
                        Lib3hSpan::fixme(),
                        RequestToParent::IncomingConnection { uri },
                    )?;
                }
                StreamEvent::ReceivedData(uri, payload) => {
                    trace!(
                        "StreamEvent::ReceivedData: {:?}",
                        String::from_utf8(payload.clone())
                    );
                    self.endpoint_self.publish(
                        Lib3hSpan::fixme(),
                        RequestToParent::ReceivedData {
                            uri,
                            payload: Opaque::from(payload),
                        },
                    )?;
                }
                StreamEvent::ConnectionClosed(uri) => {
                    trace!("StreamEvent::ConnectionClosed: {}", uri);
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
            if let RequestToChild::SendMessage { uri, payload } = inner_msg {
                if self.streams.connection_status(&uri) == ConnectionStatus::Ready {
                    trace!("Sending pending message to: {:?}", uri);
                    msg.put_message(RequestToChild::SendMessage { uri, payload });
                    if let Err(msg) = self.handle_send_message(msg) {
                        trace!("Error while sending message, putting it back in pending list");
                        temp.push(msg);
                    }
                } else {
                    msg.put_message(RequestToChild::SendMessage { uri, payload });
                    temp.push(msg);
                }
            } else {
                panic!("Found a non-SendMessage message in GhostWebsocketTransport::pending!");
            }
        }
        self.pending = temp;
        Ok(())
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

pub type GhostTransportWebsocketEndpointContextParent = GhostContextEndpoint<
    (),
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
        // process the self endpoint
        detach_run!(self.endpoint_self, |endpoint_self| endpoint_self
            .process(self))?;

        self.process_actor_inbox()?;

        // make sure we have bound and get our address if so
        let my_addr = match &self.bound_url {
            Some(my_addr) => my_addr.clone(),
            None => return Ok(false.into()),
        };

        trace!("Processing for: {}", my_addr);

        let (did_work, stream_events) = self.streams.process()?;
        self.process_stream_events(stream_events)?;
        self.process_pending_messages()?;

        Ok(did_work.into())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{tests::enable_logging_for_test, transport::websocket::tls::TlsConfig};
    use lib3h_ghost_actor::wait_for_message;
    use regex::Regex;
    use std::{net::TcpListener, thread, time};
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
    fn test_websocket_transport() {
        let machine_id1 = "fake_machine_id1".into();
        let mut transport1 = GhostTransportWebsocket::new(machine_id1, TlsConfig::Unencrypted);
        let mut t1_endpoint: GhostTransportWebsocketEndpointContextParent = transport1
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix("twss_to_child1")
            .build::<()>();

        let machine_id2 = "fake_machine_id2".into();
        let mut transport2 = GhostTransportWebsocket::new(machine_id2, TlsConfig::Unencrypted);;
        let mut t2_endpoint = transport2
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix("twss_to_child2")
            .build::<()>();

        // create two memory bindings so that we have addresses
        assert_eq!(transport1.bound_url, None);
        assert_eq!(transport2.bound_url, None);

        let port1 = get_available_port(1025).expect("Must be able to find free port");
        let expected_transport1_address =
            Url::parse(&format!("wss://127.0.0.1:{}", port1)).unwrap();
        t1_endpoint
            .request(
                Lib3hSpan::fixme(),
                RequestToChild::Bind {
                    spec: expected_transport1_address.clone(),
                },
                Box::new(move |_: &mut (), r| {
                    // parent should see the bind event
                    assert_eq!(
                        format!(
                            "Response(Ok(Bind(BindResultData {{ bound_url: \"wss://127.0.0.1:{}/\" }})))",
                            port1.clone(),
                        ),
                        format!("{:?}", r)
                    );
                    Ok(())
                }),
            )
            .unwrap();

        let port2 = get_available_port(1026).expect("Must be able to find free port");
        let expected_transport2_address =
            Url::parse(&format!("wss://127.0.0.1:{}", port2)).unwrap();
        t2_endpoint
            .request(
                Lib3hSpan::fixme(),
                RequestToChild::Bind {
                    spec: expected_transport2_address.clone(),
                },
                Box::new(move |_: &mut (), r| {
                    // parent should see the bind event
                    assert_eq!(
                        &format!(
                            "Response(Ok(Bind(BindResultData {{ bound_url: \"wss://127.0.0.1:{}/\" }})))",
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

        transport2.process().unwrap();
        let _ = t2_endpoint.process(&mut ());

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
                Lib3hSpan::fixme(),
                RequestToChild::SendMessage {
                    uri: expected_transport2_address.clone(),
                    payload: b"test message".to_vec().into(),
                },
                Box::new(|_: &mut (), r| {
                    // parent should see that the send request was OK
                    assert_eq!("Response(Ok(SendMessageSuccess))", &format!("{:?}", r));
                    Ok(())
                }),
            )
            .unwrap();

        wait_for_message!(
            vec![&mut transport1, &mut transport2],
            t2_endpoint,
            "ReceivedData \\{ uri: \"wss://127\\.0\\.0\\.1:\\d+/\", payload: \"test message\" \\}"
        );
    }

    #[test]
    fn test_websocket_transport_reconnect() {
        enable_logging_for_test(true);
        let machine_id1 = "fake_machine_id1".into();
        let mut transport1 = GhostTransportWebsocket::new(machine_id1, TlsConfig::Unencrypted);
        let mut t1_endpoint: GhostTransportWebsocketEndpointContextParent = transport1
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix("twss_to_child1")
            .build::<()>();

        let port1 = get_available_port(2025).expect("Must be able to find free port");
        let expected_transport1_address =
            Url::parse(&format!("wss://127.0.0.1:{}", port1)).unwrap();
        t1_endpoint
            .request(
                Lib3hSpan::fixme(),
                RequestToChild::Bind {
                    spec: expected_transport1_address.clone(),
                },
                Box::new(|_: &mut (), _| Ok(())),
            )
            .unwrap();

        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut ());

        assert_eq!(
            transport1.bound_url(),
            Some(expected_transport1_address.clone())
        );

        let port2 = get_available_port(2026).expect("Must be able to find free port");

        for index in 1..10 {
            transport1.process().unwrap();
            {
                let machine_id2 = "fake_machine_id2".into();
                let mut transport2 =
                    GhostTransportWebsocket::new(machine_id2, TlsConfig::Unencrypted);;
                let mut t2_endpoint = transport2
                    .take_parent_endpoint()
                    .expect("exists")
                    .as_context_endpoint_builder()
                    .request_id_prefix("twss_to_child2")
                    .build::<()>();

                let expected_transport2_address =
                    Url::parse(&format!("wss://127.0.0.1:{}", port2)).unwrap();
                t2_endpoint
                    .request(
                        Lib3hSpan::fixme(),
                        RequestToChild::Bind {
                            spec: expected_transport2_address.clone(),
                        },
                        Box::new(|_: &mut (), _| Ok(())),
                    )
                    .unwrap();
                transport2.process().unwrap();
                let _ = t2_endpoint.process(&mut ());

                assert_eq!(
                    transport2.bound_url(),
                    Some(expected_transport2_address.clone())
                );

                // now send a message from transport1 to transport2 over the bound addresses
                t1_endpoint
                    .request(
                        Lib3hSpan::fixme(),
                        RequestToChild::SendMessage {
                            uri: expected_transport2_address.clone(),
                            payload: b"test message".to_vec().into(),
                        },
                        Box::new(|_: &mut (), r| {
                            // parent should see that the send request was OK
                            assert_eq!("Response(Ok(SendMessageSuccess))", &format!("{:?}", r));
                            Ok(())
                        }),
                    )
                    .unwrap();

                wait_for_message!(
                    vec![&mut transport1, &mut transport2],
                    t2_endpoint,
                    "ReceivedData \\{ uri: \"wss://127\\.0\\.0\\.1:\\d+/\", payload: \"test message\" \\}"
                );
            }

            println!("Try {} successful!", index);
            thread::sleep(time::Duration::from_millis(1));
        }
    }
}
