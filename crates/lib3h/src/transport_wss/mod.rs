//! abstraction for working with Websocket connections
//! based on any rust io Read/Write Stream

mod tcp;

use crate::transport::{
    error::{TransportError, TransportResult},
    protocol::{RequestId, TransportCommand, TransportEvent},
    transport_trait::Transport,
};
use lib3h_protocol::DidWork;
use std::{
    collections::VecDeque,
    io::{Read, Write},
};

use url::Url;

static FAKE_PKCS12: &'static [u8] = include_bytes!("fake_key.p12");
static FAKE_PASS: &'static str = "hello";

// -- some internal types for readability -- //

type TlsConnectResult<T> = Result<TlsStream<T>, native_tls::HandshakeError<T>>;
type WsHandshakeError<T> =
    tungstenite::handshake::HandshakeError<tungstenite::handshake::client::ClientHandshake<T>>;
type WsConnectResult<T> =
    Result<(WsStream<T>, tungstenite::handshake::client::Response), WsHandshakeError<T>>;
type WsSrvHandshakeError<T> = tungstenite::handshake::HandshakeError<
    tungstenite::handshake::server::ServerHandshake<T, tungstenite::handshake::server::NoCallback>,
>;
type WsSrvAcceptResult<T> = Result<WsStream<T>, WsSrvHandshakeError<T>>;
type WssHandshakeError<T> = tungstenite::handshake::HandshakeError<
    tungstenite::handshake::client::ClientHandshake<TlsStream<T>>,
>;
type WssConnectResult<T> =
    Result<(WssStream<T>, tungstenite::handshake::client::Response), WssHandshakeError<T>>;
type WssSrvHandshakeError<T> = tungstenite::handshake::HandshakeError<
    tungstenite::handshake::server::ServerHandshake<
        TlsStream<T>,
        tungstenite::handshake::server::NoCallback,
    >,
>;
type WssSrvAcceptResult<T> = Result<WssStream<T>, WssSrvHandshakeError<T>>;
type TlsMidHandshake<T> = native_tls::MidHandshakeTlsStream<BaseStream<T>>;

type BaseStream<T> = T;
type TlsSrvMidHandshake<T> = native_tls::MidHandshakeTlsStream<BaseStream<T>>;
type TlsStream<T> = native_tls::TlsStream<BaseStream<T>>;
type WsMidHandshake<T> = tungstenite::handshake::MidHandshake<tungstenite::ClientHandshake<T>>;
type WsSrvMidHandshake<T> = tungstenite::handshake::MidHandshake<
    tungstenite::ServerHandshake<T, tungstenite::handshake::server::NoCallback>,
>;
type WssMidHandshake<T> =
    tungstenite::handshake::MidHandshake<tungstenite::ClientHandshake<TlsStream<T>>>;
type WssSrvMidHandshake<T> = tungstenite::handshake::MidHandshake<
    tungstenite::ServerHandshake<TlsStream<T>, tungstenite::handshake::server::NoCallback>,
>;
type WsStream<T> = tungstenite::protocol::WebSocket<T>;
type WssStream<T> = tungstenite::protocol::WebSocket<TlsStream<T>>;

type SocketMap<T> = std::collections::HashMap<Url, WssInfo<T>>;

// an internal state sequence for stream building
#[derive(Debug)]
enum WebsocketStreamState<T: Read + Write + std::fmt::Debug> {
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

/// how often should we send a heartbeat if we have not received msgs
pub const DEFAULT_HEARTBEAT_MS: usize = 2000;

/// when should we close a connection due to not receiving remote msgs
pub const DEFAULT_HEARTBEAT_WAIT_MS: usize = 5000;

/// Represents an individual connection
#[derive(Debug)]
pub struct WssInfo<T: Read + Write + std::fmt::Debug> {
    request_id: String,
    url: url::Url,
    last_msg: std::time::Instant,
    send_queue: Vec<Vec<u8>>,
    stateful_socket: WebsocketStreamState<T>,
}

impl<T: Read + Write + std::fmt::Debug> WssInfo<T> {
    pub fn close(&mut self) -> TransportResult<()> {
        if let WebsocketStreamState::ReadyWss(socket) = &mut self.stateful_socket {
            socket.close(None)?;
            socket.write_pending()?;
        }
        self.stateful_socket = WebsocketStreamState::None;
        Ok(())
    }

    pub fn new(url: url::Url, socket: BaseStream<T>, is_server: bool) -> Self {
        WssInfo {
            request_id: "".to_string(),
            url,
            last_msg: std::time::Instant::now(),
            send_queue: Vec::new(),
            stateful_socket: match is_server {
                false => WebsocketStreamState::Connecting(socket),
                true => WebsocketStreamState::ConnectingSrv(socket),
            },
        }
    }

    pub fn client(url: url::Url, socket: BaseStream<T>) -> Self {
        Self::new(url, socket, false)
    }

    pub fn server(url: url::Url, socket: BaseStream<T>) -> Self {
        Self::new(url, socket, true)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TlsCertificate {
    pkcs12_data: Vec<u8>,
    passphrase: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TlsConfig {
    Unencrypted,
    FakeServer,
    SuppliedCertificate(TlsCertificate),
}

/// A factory callback for generating base streams of type T
pub type StreamFactory<T> = fn(uri: &str) -> TransportResult<T>;

/// A function that produces accepted sockets of type R wrapped in a TransportInfo
pub type Acceptor<T> = Box<dyn FnMut() -> TransportResult<WssInfo<T>>>;

/// A function that binds to a url and produces sockt acceptors of type T
pub type Bind<T> = Box<dyn FnMut(&Url) -> TransportResult<Acceptor<T>>>;

/// A "Transport" implementation based off the websocket protocol
/// any rust io Read/Write stream should be able to serve as the base
pub struct TransportWss<T: Read + Write + std::fmt::Debug> {
    tls_config: TlsConfig,
    stream_factory: StreamFactory<T>,
    stream_sockets: SocketMap<T>,
    event_queue: Vec<TransportEvent>,
    inbox: VecDeque<TransportCommand>,
    bind: Bind<T>,
    acceptor: TransportResult<Acceptor<T>>,
}

impl<T: Read + Write + std::fmt::Debug> Transport for TransportWss<T> {
    /// get a list of all open transport ids
    fn connection_list(&self) -> TransportResult<Vec<Url>> {
        Ok(self.stream_sockets.keys().map(|x|x.clone()).collect())
    }

    /// record message to process on "process"
    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.inbox.push_back(command);
        Ok(())
    }

    /// this should be called frequently on the event loop
    /// looks for incoming messages or processes ping/pong/close events etc
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        let mut did_work = false;

        while let Some(cmd) = self.inbox.pop_front() {
            did_work = true;
            self.serve_TransportCommand(cmd)?;
        }

        if self.priv_process_stream_sockets()? {
            did_work = true
        }

        Ok((did_work, self.event_queue.drain(..).collect()))
    }

    fn bind_sync(&mut self, spec: Url) -> TransportResult<Url> {
        let rid = nanoid::simple();
        self.bind(rid.clone(), spec);
        for _x in 0..100 {
            let (_, evt_list) = self.process()?;
            let mut out = None;
            for evt in evt_list {
                match &evt {
                    TransportEvent::BindSuccess { request_id, bound_address } => {
                        if request_id == &rid {
                            out = Some(bound_address.clone());
                        }
                        self.event_queue.push(evt);
                    }
                    _ => self.event_queue.push(evt),
                }
            }
            if out.is_some() {
                return Ok(out.unwrap());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Err(TransportError::new("bind fail".to_string().into()))
    }
}

impl<T: Read + Write + std::fmt::Debug + std::marker::Sized> TransportWss<T> {
    pub fn new(stream_factory: StreamFactory<T>, bind: Bind<T>, tls_config: TlsConfig) -> Self {
        TransportWss {
            tls_config,
            stream_factory,
            stream_sockets: std::collections::HashMap::new(),
            event_queue: Vec::new(),
            inbox: VecDeque::new(),
            bind,
            acceptor: Err(TransportError("acceptor not initialized".into())),
        }
    }

    // -- private -- //

    #[allow(non_snake_case)]
    fn serve_TransportCommand(&mut self, cmd: TransportCommand) -> TransportResult<()> {
        match cmd {
            TransportCommand::Bind { request_id, spec } => {
                self.priv_bind(request_id, spec)
            }
            TransportCommand::Connect { request_id, address } => {
                self.priv_connect(request_id, address)
            }
            TransportCommand::SendMessage { request_id, address, payload } => {
                self.priv_send(request_id, address, payload)
            }
        }
    }

    /// bind to network interface
    fn priv_bind(&mut self, request_id: RequestId, spec: Url) -> TransportResult<()> {
        let acceptor = (self.bind)(&spec);

        let bound_address = acceptor.map(|acceptor| {
            self.acceptor = Ok(acceptor);
            spec.clone()
        })?;

        self.event_queue.push(TransportEvent::BindSuccess {
            request_id,
            bound_address,
        });

        Ok(())
    }

    /// connect to a remote websocket service
    fn priv_connect(&mut self, request_id: RequestId, address: Url) -> TransportResult<()> {
        let host_port = format!(
            "{}:{}",
            address.host_str()
                .ok_or_else(|| TransportError("bad connect host".into()))?,
            address.port()
                .ok_or_else(|| TransportError("bad connect port".into()))?,
        );
        let socket = (self.stream_factory)(&host_port)?;
        let info = WssInfo::client(address.clone(), socket);
        self.stream_sockets.insert(address, info);

        Ok(())
    }

    /// send a message to one or more remote connected nodes
    fn priv_send(&mut self, request_id: RequestId, address: Url, payload: Vec<u8>) -> TransportResult<()> {
        if let Some(info) = self.stream_sockets.get_mut(&address) {
            info.send_queue.push(payload);
        }

        // TODO XXX - give success after it actually sends
        self.event_queue.push(TransportEvent::SendMessageSuccess {
            request_id
        });

        Ok(())
    }

    fn priv_process_accept(&mut self) -> DidWork {
        match &mut self.acceptor {
            Err(err) => {
                warn!("acceptor in error state: {:?}", err);
                false
            }
            Ok(acceptor) => (acceptor)()
                .map(move |wss_info| {
                    let address = wss_info.url.clone();
                    let _insert_result = self.stream_sockets.insert(address, wss_info);
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
        let sockets: Vec<(Url, WssInfo<T>)> = self.stream_sockets.drain().collect();

        for (url, mut info) in sockets {
            if let Err(e) = self.priv_process_socket(&mut did_work, &mut info) {
                self.event_queue
                    .push(TransportEvent::ConnectionError { address: info.url.clone(), error: e });
            }
            if let WebsocketStreamState::None = info.stateful_socket {
                self.event_queue
                    .push(TransportEvent::ConnectionClosed { address: info.url });
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
                    .push(TransportEvent::ConnectionClosed { address: info.url });
                info.stateful_socket = WebsocketStreamState::None;
                continue;
            }
            self.stream_sockets.insert(url, info);
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
                            &info.url,
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
                        self.priv_ws_srv_handshake(&info.url, tungstenite::accept(socket))?;
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
                    &info.url,
                    &info.request_id,
                    tungstenite::client(info.url.clone(), socket),
                )?;
                Ok(())
            }
            WebsocketStreamState::TlsSrvReady(socket) => {
                info.last_msg = std::time::Instant::now();
                *did_work = true;
                info.stateful_socket =
                    self.priv_wss_srv_handshake(&info.url, tungstenite::accept(socket))?;
                Ok(())
            }
            WebsocketStreamState::WsMidHandshake(socket) => {
                info.stateful_socket =
                    self.priv_ws_handshake(&info.url, &info.request_id, socket.handshake())?;
                Ok(())
            }
            WebsocketStreamState::WsSrvMidHandshake(socket) => {
                info.stateful_socket = self.priv_ws_srv_handshake(&info.url, socket.handshake())?;
                Ok(())
            }
            WebsocketStreamState::WssMidHandshake(socket) => {
                info.stateful_socket =
                    self.priv_wss_handshake(&info.url, &info.request_id, socket.handshake())?;
                Ok(())
            }
            WebsocketStreamState::WssSrvMidHandshake(socket) => {
                info.stateful_socket = self.priv_wss_srv_handshake(&info.url, socket.handshake())?;
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
                                .push(TransportEvent::ReceivedData { address: info.url.clone(), payload: msg });
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
                                .push(TransportEvent::ReceivedData { address: info.url.clone(), payload: msg });
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
        address: &Url,
        request_id: &str,
        res: WsConnectResult<T>,
    ) -> TransportResult<WebsocketStreamState<T>> {
        match res {
            Err(tungstenite::HandshakeError::Interrupted(socket)) => {
                Ok(WebsocketStreamState::WsMidHandshake(socket))
            }
            Err(e) => Err(e.into()),
            Ok((socket, _response)) => {
                self.event_queue.push(TransportEvent::ConnectSuccess {
                    request_id: request_id.to_string(),
                    address: address.clone(),
                });
                Ok(WebsocketStreamState::ReadyWs(Box::new(socket)))
            }
        }
    }

    // process websocket handshaking
    fn priv_wss_handshake(
        &mut self,
        address: &Url,
        request_id: &str,
        res: WssConnectResult<T>,
    ) -> TransportResult<WebsocketStreamState<T>> {
        match res {
            Err(tungstenite::HandshakeError::Interrupted(socket)) => {
                Ok(WebsocketStreamState::WssMidHandshake(socket))
            }
            Err(e) => Err(e.into()),
            Ok((socket, _response)) => {
                self.event_queue.push(TransportEvent::ConnectSuccess {
                    request_id: request_id.to_string(),
                    address: address.clone(),
                });
                Ok(WebsocketStreamState::ReadyWss(Box::new(socket)))
            }
        }
    }

    // process websocket srv handshaking
    fn priv_ws_srv_handshake(
        &mut self,
        address: &Url,
        res: WsSrvAcceptResult<T>,
    ) -> TransportResult<WebsocketStreamState<T>> {
        match res {
            Err(tungstenite::HandshakeError::Interrupted(socket)) => {
                Ok(WebsocketStreamState::WsSrvMidHandshake(socket))
            }
            Err(e) => Err(e.into()),
            Ok(socket) => {
                self.event_queue
                    .push(TransportEvent::IncomingConnection { address: address.clone() });
                Ok(WebsocketStreamState::ReadyWs(Box::new(socket)))
            }
        }
    }

    // process websocket srv handshaking
    fn priv_wss_srv_handshake(
        &mut self,
        address: &Url,
        res: WssSrvAcceptResult<T>,
    ) -> TransportResult<WebsocketStreamState<T>> {
        match res {
            Err(tungstenite::HandshakeError::Interrupted(socket)) => {
                Ok(WebsocketStreamState::WssSrvMidHandshake(socket))
            }
            Err(e) => Err(e.into()),
            Ok(socket) => {
                self.event_queue
                    .push(TransportEvent::IncomingConnection { address: address.clone() });
                Ok(WebsocketStreamState::ReadyWss(Box::new(socket)))
            }
        }
    }
}
