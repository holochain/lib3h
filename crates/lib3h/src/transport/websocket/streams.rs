use crate::transport::{
    error::{TransportError, TransportResult},
    websocket::{
        BaseStream, tls::TlsConfig, TlsMidHandshake, TlsSrvMidHandshake, TlsStream,
        wss_info::WssInfo, WsMidHandshake, WssMidHandshake, WsSrvMidHandshake, WssSrvMidHandshake,
        WsStream, SocketMap, WssStream, FAKE_PKCS12, FAKE_PASS, TlsConnectResult, WsConnectResult,
        WssConnectResult, WsSrvAcceptResult, WssSrvAcceptResult
    },
};
use lib3h_protocol::DidWork;
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};

use url::Url;

/// how often should we send a heartbeat if we have not received msgs
pub const DEFAULT_HEARTBEAT_MS: usize = 2000;

/// when should we close a connection due to not receiving remote msgs
pub const DEFAULT_HEARTBEAT_WAIT_MS: usize = 5000;

/// a connection identifier
pub type ConnectionId = String;
pub type ConnectionIdRef = str;


// an internal state sequence for stream building
#[derive(Debug)]
pub enum WebsocketStreamState<T: Read + Write + std::fmt::Debug> {
    None,
    Connecting(BaseStream<T>),
    #[allow(dead_code)]
    ConnectingSrv(BaseStream<T>),
    TlsMidHandshake(TlsMidHandshake<T>),
    TlsSrvMidHandshake(TlsSrvMidHandshake<T>),
    TlsReady(TlsStream<T>),
    TlsSrvReady(TlsStream<T>),
    WsMidHandshake(WsMidHandshake<T>),
    WsSrvMidHandshake(WsSrvMidHandshake<T>),
    WssMidHandshake(WssMidHandshake<T>),
    WssSrvMidHandshake(WssSrvMidHandshake<T>),
    ReadyWs(Box<WsStream<T>>),
    ReadyWss(Box<WssStream<T>>),
}


/// Events that can be generated during a `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum StreamEvent {
    /// Notify that some TransportError occured
    ErrorOccured(ConnectionId, TransportError),
    /// an outgoing connection has been established
    ConnectResult(ConnectionId, String),
    /// we have received an incoming connection
    IncomingConnectionEstablished(ConnectionId),
    /// We have received data from a connection
    ReceivedData(ConnectionId, Vec<u8>),
    /// A connection closed for whatever reason
    ConnectionClosed(ConnectionId),
}

/// A factory callback for generating base streams of type T
pub type StreamFactory<T> = fn(uri: &str) -> TransportResult<T>;

pub trait IdGenerator {
    fn next_id(&mut self) -> ConnectionId;
}

#[derive(Clone)]
/// A ConnectionIdFactory is a tuple of it's own internal id and incremental counter
/// for each established transport
pub struct ConnectionIdFactory(u64, Arc<Mutex<u64>>);
impl IdGenerator for ConnectionIdFactory {
    fn next_id(&mut self) -> ConnectionId {
        let self_id = self.0;
        let mut n_id = self.1.lock().expect("could not lock mutex");
        let out = format!("ws{}_{}", self_id, *n_id);
        *n_id += 1;
        out
    }
}

lazy_static! {
    static ref TRANSPORT_COUNT: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
}
impl ConnectionIdFactory {
    pub fn new() -> Self {
        let mut tc = TRANSPORT_COUNT
            .lock()
            .expect("could not lock transport count mutex");
        *tc += 1;
        ConnectionIdFactory(*tc, Arc::new(Mutex::new(1)))
    }
}

/// A function that produces accepted sockets of type R wrapped in a TransportInfo
pub type Acceptor<T> = Box<dyn FnMut(ConnectionIdFactory) -> TransportResult<WssInfo<T>>>;

/// A function that binds to a url and produces sockt acceptors of type T
pub type Bind<T> = Box<dyn FnMut(&Url) -> TransportResult<Acceptor<T>>>;

/// A "Transport" implementation based off the websocket protocol
/// any rust io Read/Write stream should be able to serve as the base
pub struct StreamManager<T: Read + Write + std::fmt::Debug> {
    tls_config: TlsConfig,
    stream_factory: StreamFactory<T>,
    stream_sockets: SocketMap<T>,
    event_queue: Vec<StreamEvent>,
    n_id: ConnectionIdFactory,
    bind: Bind<T>,
    acceptor: TransportResult<Acceptor<T>>,
}

impl<T: Read + Write + std::fmt::Debug> StreamManager<T> {
    pub fn new(stream_factory: StreamFactory<T>, bind: Bind<T>, tls_config: TlsConfig) -> Self {
        StreamManager {
            tls_config,
            stream_factory,
            stream_sockets: std::collections::HashMap::new(),
            event_queue: Vec::new(),
            n_id: ConnectionIdFactory::new(),
            bind,
            acceptor: Err(TransportError("acceptor not initialized".into())),
        }
    }

    pub fn url_to_connection_id(&self, url: &Url) -> Option<ConnectionId> {
        for (connection_id, wss_info) in self.stream_sockets.iter() {
            if wss_info.url == *url {
                return Some(connection_id.clone());
            }
        }
        None
    }

    pub fn connection_id_to_url(&self, connection_id: ConnectionId) -> Option<Url> {
        self.stream_sockets
            .get(&connection_id)
            .map(|wss_info| wss_info.url.clone())
    }

    /// connect to a remote websocket service
    pub fn connect(&mut self, uri: &Url) -> TransportResult<ConnectionId> {
        let host_port = format!(
            "{}:{}",
            uri.host_str()
                .ok_or_else(|| TransportError("bad connect host".into()))?,
            uri.port()
                .ok_or_else(|| TransportError("bad connect port".into()))?,
        );
        let socket = (self.stream_factory)(&host_port)?;
        let id = self.priv_next_id();
        let info = WssInfo::client(id.clone(), uri.clone(), socket);
        self.stream_sockets.insert(id.clone(), info);
        Ok(id)
    }

    /// close a currently tracked connection
    #[allow(dead_code)]
    fn close(&mut self, id: &ConnectionIdRef) -> TransportResult<()> {
        if let Some(mut info) = self.stream_sockets.remove(id) {
            info.close()?;
        }
        Ok(())
    }

    /// close all currently tracked connections
    #[allow(dead_code)]
    fn close_all(&mut self) -> TransportResult<()> {
        let mut errors: Vec<TransportError> = Vec::new();

        while !self.stream_sockets.is_empty() {
            let key = self
                .stream_sockets
                .keys()
                .next()
                .expect("should not be None")
                .to_string();
            if let Some(mut info) = self.stream_sockets.remove(&key) {
                if let Err(e) = info.close() {
                    errors.push(e);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.into())
        }
    }

    /// this should be called frequently on the event loop
    /// looks for incoming messages or processes ping/pong/close events etc
    pub fn process(&mut self) -> TransportResult<(DidWork, Vec<StreamEvent>)> {
        let mut did_work = false;

        if self.priv_process_stream_sockets()? {
            did_work = true
        }

        Ok((did_work, self.event_queue.drain(..).collect()))
    }

    /// send a message to one or more remote connected nodes
    pub fn send(&mut self, id_list: &[&ConnectionIdRef], payload: &[u8]) -> TransportResult<()> {
        for id in id_list {
            if let Some(info) = self.stream_sockets.get_mut(&id.to_string()) {
                info.send_queue.push(payload.to_vec());
            }
        }

        Ok(())
    }

    /// send a message to all remote nodes
    #[allow(dead_code)]
    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        for info in self.stream_sockets.values_mut() {
            info.send_queue.push(payload.to_vec());
        }
        Ok(())
    }

    pub fn bind(&mut self, url: &Url) -> TransportResult<Url> {
        let acceptor = (self.bind)(&url.clone());
        acceptor.map(|acceptor| {
            self.acceptor = Ok(acceptor);
            url.clone()
        })
    }

    // -- private -- //

    // generate a unique id for
    fn priv_next_id(&mut self) -> ConnectionId {
        self.n_id.next_id()
    }

    fn priv_process_accept(&mut self) -> DidWork {
        match &mut self.acceptor {
            Err(err) => {
                warn!("acceptor in error state: {:?}", err);
                false
            }
            Ok(acceptor) => (acceptor)(self.n_id.clone())
                .map(move |wss_info| {
                    let connection_id = wss_info.id.clone();
                    let _insert_result = self.stream_sockets.insert(connection_id, wss_info);
                    true
                })
                .unwrap_or_else(|err| {
                    warn!("did not accept any connections: {:?}", err);
                    false
                }),
        }
    }

    // see if any work needs to be done on our stream sockets
    fn priv_process_stream_sockets(&mut self) -> TransportResult<DidWork> {
        let mut did_work = false;

        // accept some incoming connections
        did_work |= self.priv_process_accept();

        // take sockets out, so we can mut ref into self and it at same time
        let sockets: Vec<(String, WssInfo<T>)> = self.stream_sockets.drain().collect();

        for (id, mut info) in sockets {
            if let Err(e) = self.priv_process_socket(&mut did_work, &mut info) {
                self.event_queue
                    .push(StreamEvent::ErrorOccured(info.id.clone(), e));
            }
            if let WebsocketStreamState::None = info.stateful_socket {
                self.event_queue
                    .push(StreamEvent::ConnectionClosed(info.id));
                continue;
            }
            if info.last_msg.elapsed().as_millis() as usize > DEFAULT_HEARTBEAT_MS {
                if let WebsocketStreamState::ReadyWss(socket) = &mut info.stateful_socket {
                    socket.write_message(tungstenite::Message::Ping(vec![]))?;
                }
                if let WebsocketStreamState::ReadyWs(socket) = &mut info.stateful_socket {
                    socket.write_message(tungstenite::Message::Ping(vec![]))?;
                }
            } else if info.last_msg.elapsed().as_millis() as usize > DEFAULT_HEARTBEAT_WAIT_MS {
                self.event_queue
                    .push(StreamEvent::ConnectionClosed(info.id));
                info.stateful_socket = WebsocketStreamState::None;
                continue;
            }
            self.stream_sockets.insert(id, info);
        }

        Ok(did_work)
    }

    // process the state machine of an individual socket stream
    fn priv_process_socket(
        &mut self,
        did_work: &mut bool,
        info: &mut WssInfo<T>,
    ) -> TransportResult<()> {
        // move the socket out, to be replaced
        let socket = std::mem::replace(&mut info.stateful_socket, WebsocketStreamState::None);

        debug!("transport_wss: socket={:?}", socket);
        // TODO remove?
        std::io::stdout().flush().ok().expect("flush stdout");
        match socket {
            WebsocketStreamState::None => {
                // stream must have closed, do nothing
                Ok(())
            }
            WebsocketStreamState::Connecting(socket) => {
                info.last_msg = std::time::Instant::now();
                *did_work = true;
                match &self.tls_config {
                    TlsConfig::Unencrypted => {
                        info.stateful_socket = self.priv_ws_handshake(
                            &info.id,
                            &info.request_id,
                            tungstenite::client(info.url.clone(), socket),
                        )?;
                    }
                    _ => {
                        let connector = native_tls::TlsConnector::builder()
                            .danger_accept_invalid_certs(true)
                            .danger_accept_invalid_hostnames(true)
                            .build()
                            .expect("failed to build TlsConnector");
                        info.stateful_socket =
                            self.priv_tls_handshake(connector.connect(info.url.as_str(), socket))?;
                    }
                }
                Ok(())
            }
            WebsocketStreamState::ConnectingSrv(socket) => {
                info.last_msg = std::time::Instant::now();
                *did_work = true;
                if let &TlsConfig::Unencrypted = &self.tls_config {
                    info.stateful_socket =
                        self.priv_ws_srv_handshake(&info.id, tungstenite::accept(socket))?;
                    return Ok(());
                }
                let ident = match &self.tls_config {
                    TlsConfig::Unencrypted => unimplemented!(),
                    TlsConfig::FakeServer => {
                        native_tls::Identity::from_pkcs12(FAKE_PKCS12, FAKE_PASS)?
                    }
                    TlsConfig::SuppliedCertificate(cert) => {
                        native_tls::Identity::from_pkcs12(&cert.pkcs12_data, &cert.passphrase)?
                    }
                };
                let acceptor = native_tls::TlsAcceptor::builder(ident)
                    .build()
                    .expect("failed to build TlsAcceptor");
                info.stateful_socket = self.priv_tls_srv_handshake(acceptor.accept(socket))?;
                Ok(())
            }
            WebsocketStreamState::TlsMidHandshake(socket) => {
                info.stateful_socket = self.priv_tls_handshake(socket.handshake())?;
                Ok(())
            }
            WebsocketStreamState::TlsSrvMidHandshake(socket) => {
                info.stateful_socket = self.priv_tls_srv_handshake(socket.handshake())?;
                Ok(())
            }
            WebsocketStreamState::TlsReady(socket) => {
                info.last_msg = std::time::Instant::now();
                *did_work = true;
                info.stateful_socket = self.priv_wss_handshake(
                    &info.id,
                    &info.request_id,
                    tungstenite::client(info.url.clone(), socket),
                )?;
                Ok(())
            }
            WebsocketStreamState::TlsSrvReady(socket) => {
                info.last_msg = std::time::Instant::now();
                *did_work = true;
                info.stateful_socket =
                    self.priv_wss_srv_handshake(&info.id, tungstenite::accept(socket))?;
                Ok(())
            }
            WebsocketStreamState::WsMidHandshake(socket) => {
                info.stateful_socket =
                    self.priv_ws_handshake(&info.id, &info.request_id, socket.handshake())?;
                Ok(())
            }
            WebsocketStreamState::WsSrvMidHandshake(socket) => {
                info.stateful_socket = self.priv_ws_srv_handshake(&info.id, socket.handshake())?;
                Ok(())
            }
            WebsocketStreamState::WssMidHandshake(socket) => {
                info.stateful_socket =
                    self.priv_wss_handshake(&info.id, &info.request_id, socket.handshake())?;
                Ok(())
            }
            WebsocketStreamState::WssSrvMidHandshake(socket) => {
                info.stateful_socket = self.priv_wss_srv_handshake(&info.id, socket.handshake())?;
                Ok(())
            }
            WebsocketStreamState::ReadyWs(mut socket) => {
                // This seems to be wrong. Messages shouldn't be drained.
                let msgs: Vec<Vec<u8>> = info.send_queue.drain(..).collect();
                for msg in msgs {
                    // TODO: fix this line! if there is an error, all the remaining messages will be lost!
                    socket.write_message(tungstenite::Message::Binary(msg))?;
                }

                match socket.read_message() {
                    Err(tungstenite::error::Error::Io(e)) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            info.stateful_socket = WebsocketStreamState::ReadyWs(socket);
                            return Ok(());
                        }
                        Err(e.into())
                    }
                    Err(tungstenite::error::Error::ConnectionClosed(_)) => {
                        // close event will be published
                        Ok(())
                    }
                    Err(e) => Err(e.into()),
                    Ok(msg) => {
                        info.last_msg = std::time::Instant::now();
                        *did_work = true;
                        let qmsg = match msg {
                            tungstenite::Message::Text(s) => Some(s.into_bytes()),
                            tungstenite::Message::Binary(b) => Some(b),
                            _ => None,
                        };

                        if let Some(msg) = qmsg {
                            self.event_queue
                                .push(StreamEvent::ReceivedData(info.id.clone(), msg));
                        }
                        info.stateful_socket = WebsocketStreamState::ReadyWs(socket);
                        Ok(())
                    }
                }
            }
            WebsocketStreamState::ReadyWss(mut socket) => {
                // This seems to be wrong. Messages shouldn't be drained.
                let msgs: Vec<Vec<u8>> = info.send_queue.drain(..).collect();
                for msg in msgs {
                    // TODO: fix this line! if there is an error, all the remaining messages will be lost!
                    socket.write_message(tungstenite::Message::Binary(msg))?;
                }

                match socket.read_message() {
                    Err(tungstenite::error::Error::Io(e)) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            info.stateful_socket = WebsocketStreamState::ReadyWss(socket);
                            return Ok(());
                        }
                        Err(e.into())
                    }
                    Err(tungstenite::error::Error::ConnectionClosed(_)) => {
                        // close event will be published
                        Ok(())
                    }
                    Err(e) => Err(e.into()),
                    Ok(msg) => {
                        info.last_msg = std::time::Instant::now();
                        *did_work = true;
                        let qmsg = match msg {
                            tungstenite::Message::Text(s) => Some(s.into_bytes()),
                            tungstenite::Message::Binary(b) => Some(b),
                            _ => None,
                        };

                        if let Some(msg) = qmsg {
                            self.event_queue
                                .push(StreamEvent::ReceivedData(info.id.clone(), msg));
                        }
                        info.stateful_socket = WebsocketStreamState::ReadyWss(socket);
                        Ok(())
                    }
                }
            }
        }
    }

    // process tls handshaking
    fn priv_tls_handshake(
        &mut self,
        res: TlsConnectResult<T>,
    ) -> TransportResult<WebsocketStreamState<T>> {
        match res {
            Err(native_tls::HandshakeError::WouldBlock(socket)) => {
                Ok(WebsocketStreamState::TlsMidHandshake(socket))
            }
            Err(e) => Err(e.into()),
            Ok(socket) => Ok(WebsocketStreamState::TlsReady(socket)),
        }
    }

    // process tls handshaking
    fn priv_tls_srv_handshake(
        &mut self,
        res: TlsConnectResult<T>,
    ) -> TransportResult<WebsocketStreamState<T>> {
        trace!("[t] processing tls connect result: {:?}", res);
        match res {
            Err(native_tls::HandshakeError::WouldBlock(socket)) => {
                Ok(WebsocketStreamState::TlsSrvMidHandshake(socket))
            }
            Err(e) => Err(e.into()),
            Ok(socket) => Ok(WebsocketStreamState::TlsSrvReady(socket)),
        }
    }

    // process websocket handshaking
    fn priv_ws_handshake(
        &mut self,
        id: &ConnectionId,
        request_id: &str,
        res: WsConnectResult<T>,
    ) -> TransportResult<WebsocketStreamState<T>> {
        match res {
            Err(tungstenite::HandshakeError::Interrupted(socket)) => {
                Ok(WebsocketStreamState::WsMidHandshake(socket))
            }
            Err(e) => Err(e.into()),
            Ok((socket, _response)) => {
                self.event_queue.push(StreamEvent::ConnectResult(
                    id.clone(),
                    request_id.to_string(),
                ));
                Ok(WebsocketStreamState::ReadyWs(Box::new(socket)))
            }
        }
    }

    // process websocket handshaking
    fn priv_wss_handshake(
        &mut self,
        id: &ConnectionId,
        request_id: &str,
        res: WssConnectResult<T>,
    ) -> TransportResult<WebsocketStreamState<T>> {
        match res {
            Err(tungstenite::HandshakeError::Interrupted(socket)) => {
                Ok(WebsocketStreamState::WssMidHandshake(socket))
            }
            Err(e) => Err(e.into()),
            Ok((socket, _response)) => {
                self.event_queue.push(StreamEvent::ConnectResult(
                    id.clone(),
                    request_id.to_string(),
                ));
                Ok(WebsocketStreamState::ReadyWss(Box::new(socket)))
            }
        }
    }

    // process websocket srv handshaking
    fn priv_ws_srv_handshake(
        &mut self,
        id: &ConnectionId,
        res: WsSrvAcceptResult<T>,
    ) -> TransportResult<WebsocketStreamState<T>> {
        match res {
            Err(tungstenite::HandshakeError::Interrupted(socket)) => {
                Ok(WebsocketStreamState::WsSrvMidHandshake(socket))
            }
            Err(e) => Err(e.into()),
            Ok(socket) => {
                self.event_queue
                    .push(StreamEvent::IncomingConnectionEstablished(id.clone()));
                Ok(WebsocketStreamState::ReadyWs(Box::new(socket)))
            }
        }
    }

    // process websocket srv handshaking
    fn priv_wss_srv_handshake(
        &mut self,
        id: &ConnectionId,
        res: WssSrvAcceptResult<T>,
    ) -> TransportResult<WebsocketStreamState<T>> {
        match res {
            Err(tungstenite::HandshakeError::Interrupted(socket)) => {
                Ok(WebsocketStreamState::WssSrvMidHandshake(socket))
            }
            Err(e) => Err(e.into()),
            Ok(socket) => {
                self.event_queue
                    .push(StreamEvent::IncomingConnectionEstablished(id.clone()));
                Ok(WebsocketStreamState::ReadyWss(Box::new(socket)))
            }
        }
    }
}
