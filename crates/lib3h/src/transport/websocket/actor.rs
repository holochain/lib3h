use crate::transport::{
    error::TransportError,
    protocol::*,
    websocket::{
        streams::{StreamEvent, StreamManager},
        tls::TlsConfig,
    },
};
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::data_types::Opaque;
use lib3h_tracing::Lib3hSpan;
use url::Url;

pub struct GhostTransportWebsocket {
    endpoint_parent: Option<GhostTransportWebsocketEndpoint>,
    endpoint_self: Option<GhostTransportWebsocketEndpointContext>,
    streams: StreamManager<std::net::TcpStream>,
    bound_url: Option<Url>,
}

impl GhostTransportWebsocket {
    pub fn new(tls_config: TlsConfig) -> GhostTransportWebsocket {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        GhostTransportWebsocket {
            endpoint_parent: Some(endpoint_parent),
            endpoint_self: Some(
                endpoint_self
                    .as_context_endpoint_builder()
                    .request_id_prefix("twss_to_parent")
                    .build(),
            ),
            streams: StreamManager::with_std_tcp_stream(tls_config),
            bound_url: None,
        }
    }

    pub fn bound_url(&self) -> Option<Url> {
        self.bound_url.clone()
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
        let mut endpoint_self = std::mem::replace(&mut self.endpoint_self, None);
        endpoint_self.as_mut().expect("exists").process(self)?;
        std::mem::replace(&mut self.endpoint_self, endpoint_self);

        for mut msg in self
            .endpoint_self
            .as_mut()
            .expect("exists")
            .drain_messages()
        {
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
                    match self.bound_url.clone() {
                        None => {
                            msg.respond(Err(TransportError::new(
                                "Transport must be bound before sending".to_string(),
                            )))?;
                        }
                        Some(my_addr) => {
                            // Trying to find established connection for URI:
                            let mut maybe_connection_id = self.streams.url_to_connection_id(&uri);

                            // If there is none, try to connect:
                            if maybe_connection_id.is_none() {
                                trace!(
                                        "No open connection to {} found when sending data. Trying to connect...",
                                        uri,
                                    );
                                match self.streams.connect(&uri) {
                                    Ok(connection_id) => {
                                        maybe_connection_id = Some(connection_id);
                                        trace!("New connection to {} established", uri.to_string());
                                    }
                                    Err(error) => {
                                        trace!(
                                            "Could not connect to {}! Transport error: {:?}",
                                            uri.to_string(),
                                            error
                                        );
                                    }
                                }
                            }

                            // If we have a connection now, send payload:
                            if let Some(connection_id) = maybe_connection_id {
                                trace!(
                                    "(GhostTransportWebsocket).SendMessage from {} to  {} | {:?}",
                                    my_addr,
                                    uri,
                                    payload
                                );
                                // Send it data from us
                                match self.streams.send(&[&connection_id], &payload.as_bytes()) {
                                    Ok(()) => {
                                        msg.respond(Ok(RequestToChildResponse::SendMessageSuccess))?
                                    }
                                    Err(error) => msg.respond(Err(error))?,
                                }
                            } else {
                                msg.respond(Err(TransportError::new(format!(
                                    "Unable to acquire connection. Can't send to {}",
                                    uri,
                                ))))?;
                            }
                        }
                    };
                }
            }
        }

        // make sure we have bound and get our address if so
        let my_addr = match &self.bound_url {
            Some(my_addr) => my_addr.clone(),
            None => return Ok(false.into()),
        };

        println!("Processing for: {}", my_addr);

        let (did_work, stream_events) = self.streams.process()?;
        for event in stream_events {
            match event {
                StreamEvent::ErrorOccured(connection_id, error) => {
                    let uri = self
                        .streams
                        .connection_id_to_url(connection_id)
                        .expect("There must be a URL for any existing connection ID");
                    let mut endpoint_self = std::mem::replace(&mut self.endpoint_self, None);
                    endpoint_self.as_mut().expect("exists").publish(
                        Lib3hSpan::todo(),
                        RequestToParent::ErrorOccured { uri, error },
                    )?;
                    std::mem::replace(&mut self.endpoint_self, endpoint_self);
                }
                StreamEvent::ConnectResult(_connection_id, _) => {}
                StreamEvent::IncomingConnectionEstablished(connection_id) => {
                    let uri = self
                        .streams
                        .connection_id_to_url(connection_id)
                        .expect("There must be a URL for any existing connection ID");
                    let mut endpoint_self = std::mem::replace(&mut self.endpoint_self, None);
                    endpoint_self.as_mut().expect("exists").publish(
                        Lib3hSpan::todo(),
                        RequestToParent::IncomingConnection { uri },
                    )?;
                    std::mem::replace(&mut self.endpoint_self, endpoint_self);
                }
                StreamEvent::ReceivedData(connection_id, payload) => {
                    //println!("DATA RECEIVED!!! {:?}", String::from_utf8(payload.clone()));
                    let uri = self
                        .streams
                        .connection_id_to_url(connection_id)
                        .expect("There must be a URL for any existing connection ID");
                    let mut endpoint_self = std::mem::replace(&mut self.endpoint_self, None);
                    endpoint_self.as_mut().expect("exists").publish(
                        Lib3hSpan::todo(),
                        RequestToParent::ReceivedData {
                            uri,
                            payload: Opaque::from(payload),
                        },
                    )?;
                    std::mem::replace(&mut self.endpoint_self, endpoint_self);
                }
                StreamEvent::ConnectionClosed(_connection_id) => {}
            }
        }

        Ok(did_work.into())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::transport::websocket::tls::TlsConfig;
    use regex::Regex;
    use url::Url;
    //use protocol::RequestToChildResponse;
    //    use lib3h_ghost_actor::GhostCallbackData;

    #[test]
    fn test_ghost_websocket_transport() {
        /* Possible other ways we might think of setting up
               constructors for actor/parent_context_endpoint pairs:

            let (transport1_endpoint, child) = ghost_create_endpoint();
            let transport1_engine = GhostTransportMemoryEngine::new(child);

            enum TestContex {
        }

            let mut transport1_actor = GhostLocalActor::new::<TestTrace>(
            transport1_engine, transport1_endpoint);
             */

        /*
            let mut transport1 = GhostParentContextEndpoint::with_cb(|child| {
            GhostTransportMemory::new(child)
        });

            let mut transport1 = GhostParentContextEndpoint::new(
            Box::new(GhostTransportMemory::new()));
             */

        let mut transport1 = GhostTransportWebsocket::new(TlsConfig::Unencrypted);
        let mut t1_endpoint: GhostTransportWebsocketEndpointContextParent = transport1
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix("twss_to_child1")
            .build::<()>();

        let mut transport2 = GhostTransportWebsocket::new(TlsConfig::Unencrypted);;
        let mut t2_endpoint = transport2
            .take_parent_endpoint()
            .expect("exists")
            .as_context_endpoint_builder()
            .request_id_prefix("twss_to_child2")
            .build::<()>();

        // create two memory bindings so that we have addresses
        assert_eq!(transport1.bound_url, None);
        assert_eq!(transport2.bound_url, None);

        let expected_transport1_address = Url::parse("wss://0.0.0.0:22888").unwrap();
        t1_endpoint
            .request(
                Lib3hSpan::todo(),
                RequestToChild::Bind {
                    spec: expected_transport1_address.clone(),
                },
                Box::new(|_: &mut (), r| {
                    // parent should see the bind event
                    assert_eq!(
                        "Response(Ok(Bind(BindResultData { bound_url: \"wss://0.0.0.0:22888/\" })))",
                        &format!("{:?}", r)
                    );
                    Ok(())
                }),
            )
            .unwrap();
        let expected_transport2_address = Url::parse("wss://0.0.0.0:22889").unwrap();
        t2_endpoint
            .request(
                Lib3hSpan::todo(),
                RequestToChild::Bind {
                    spec: expected_transport2_address.clone(),
                },
                Box::new(|_: &mut (), r| {
                    // parent should see the bind event
                    assert_eq!(
                        "Response(Ok(Bind(BindResultData { bound_url: \"wss://0.0.0.0:22889/\" })))",
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
                Lib3hSpan::todo(),
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

        transport1.process().unwrap();
        let _ = t1_endpoint.process(&mut ());

        transport2.process().unwrap();
        let _ = t2_endpoint.process(&mut ());

        let mut requests = t2_endpoint.drain_messages();
        //assert_eq!(2, requests.len());
        assert!(format!("{:?}", requests[0].take_message())
            .starts_with("Some(IncomingConnection { uri: \"wss://127.0.0.1"));

        transport1.process().unwrap();
        transport1.process().unwrap();

        let _ = t1_endpoint.process(&mut ());

        transport2.process().unwrap();
        transport2.process().unwrap();
        let _ = t2_endpoint.process(&mut ());

        let mut requests = t2_endpoint.drain_messages();
        let message_string = format!("{:?}", requests[0].take_message());
        println!("{}", message_string);
        assert!(
            Regex::new(
                "Some\\(ReceivedData \\{ uri: \"wss://127\\.0\\.0\\.1:\\d+/\", payload: \"test message\" \\}\\)",
            ).unwrap()
            .is_match(&message_string)
        );
    }
}
