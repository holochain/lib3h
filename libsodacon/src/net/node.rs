use error;
use hex;
use libsodacrypt;
use net::message;
use net::http;
use rmp_serde;
use std;
use std::collections::{hash_map, HashMap};
use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq)]
pub struct Endpoint {
    pub addr: String,
    pub port: u16,
}

impl Endpoint {
    pub fn new (addr: &str, port: u16) -> Self {
        Endpoint {
            addr: addr.to_string(),
            port: port
        }
    }

    pub fn to_socket_addr (&self) -> error::Result<std::net::SocketAddr> {
        let s: String = format!("{}:{}", self.addr, self.port);
        let out: std::net::SocketAddr = s.parse()?;
        Ok(out)
    }
}

impl From<std::net::SocketAddr> for Endpoint {
    fn from(addr: std::net::SocketAddr) -> Self {
        Endpoint {
            addr: match addr {
                std::net::SocketAddr::V4(a) => a.ip().to_string(),
                std::net::SocketAddr::V6(a) => format!("[{}]", a.ip().to_string()),
            },
            port: addr.port(),
        }
    }
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.addr, self.port)
    }
}

#[derive(Debug)]
pub enum ServerEvent {
    OnError(error::Error),
    OnListening(Endpoint),
    OnConnection(Vec<u8>, Endpoint),
    OnDataReceived(Vec<u8>),
    OnClose(),
}

#[derive(Debug)]
pub enum ClientEvent {
    OnError(error::Error),
    OnConnected(Vec<u8>, Endpoint),
    OnDataReceived(Vec<u8>),
    OnClose(),
}

#[derive(Debug)]
pub enum Event {
    OnServerEvent(ServerEvent),
    OnClientEvent(ClientEvent),
}

struct StdNetListenCon {
    socket: std::net::TcpListener,
}

impl StdNetListenCon {
    fn new (socket: std::net::TcpListener) -> Self {
        StdNetListenCon {
            socket: socket,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum SessionState {
    New,
    WaitPing,
    Ready,
}

struct SessionClient {
    local_node_id: Vec<u8>,
    remote_node_id: Vec<u8>,
    endpoint: Endpoint,
    state: SessionState,
    eph_pub: Vec<u8>,
    eph_priv: Vec<u8>,
    key_send: Vec<u8>,
    key_recv: Vec<u8>,
    cur_socket: Option<std::net::TcpStream>,
    cur_response: http::Request,
}

impl SessionClient {
    pub fn new (local_node_id: &[u8], endpoint: &Endpoint) -> error::Result<Self> {
        let (key_pub, key_priv) = libsodacrypt::kx::gen_keypair()?;
        Ok(SessionClient {
            local_node_id: local_node_id.to_vec(),
            remote_node_id: Vec::new(),
            endpoint: endpoint.clone(),
            state: SessionState::New,
            eph_pub: key_pub,
            eph_priv: key_priv,
            key_send: Vec::new(),
            key_recv: Vec::new(),
            cur_socket: None,
            cur_response: http::Request::new(http::RequestType::Response),
        })
    }

    pub fn ping (&mut self) -> error::Result<()> {
        let mut socket = wrap_connect(&self.endpoint)?;

        let ping_req = message::PingReq::new();

        let out = message::compile(
            &self.local_node_id,
            &vec![message::Message::PingReq(Box::new(ping_req))],
            http::RequestType::Request)?;

        socket.write(&out)?;

        self.cur_socket = Some(socket);

        Ok(())
    }

    pub fn process_once (mut self) -> (Option<Self>, Vec<Event>) {
        let mut buf = [0u8; 1024];
        let mut events: Vec<Event> = Vec::new();

        let mut socket = match self.cur_socket.take() {
            None => return (Some(self), events),
            Some(s) => s
        };

        let size = match socket.read(&mut buf) {
            Ok(b) => {
                if b < 1 {
                    events.push(Event::OnClientEvent(ClientEvent::OnClose()));
                    return (Some(self), events);
                } else {
                    b
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                self.cur_socket = Some(socket);
                return (Some(self), events);
            }
            Err(e) => {
                events.push(Event::OnClientEvent(ClientEvent::OnError(error::Error::from(e))));
                events.push(Event::OnClientEvent(ClientEvent::OnClose()));
                return (None, events);
            }
        };

        {
            if !self.cur_response.check_parse(&buf[..size]) {
                self.cur_socket = Some(socket);
                return (Some(self), events);
            }
        }

        let response = self.cur_response;
        self.cur_response = http::Request::new(http::RequestType::Response);
        println!("GOT RESPONSE: {:?}", response);

        match self.state {
            SessionState::New => {
                self.process_initial_handshake(events, response)
            }
            _ => {
                panic!("ahh, cant handle this yet: {:?}", self.state);
            }
        }
    }

    // -- private -- //

    fn process_initial_handshake (mut self, mut events: Vec<Event>, response: http::Request) -> (Option<Self>, Vec<Event>) {
        let (mut cli_recv, mut cli_send, mut remote_node_id) = match wrap_parse_initial_handshake(&response.body, &self.eph_pub, &self.eph_priv) {
            Ok(v) => v,
            Err(e) => {
                events.push(Event::OnClientEvent(ClientEvent::OnError(error::Error::from(e))));
                events.push(Event::OnClientEvent(ClientEvent::OnClose()));
                return (None, events);
            }
        };

        self.remote_node_id.append(&mut remote_node_id);
        self.eph_pub.drain(..);
        self.eph_priv.drain(..);
        self.key_send.append(&mut cli_send);

        self.key_recv.append(&mut cli_recv);

        self.state = SessionState::WaitPing;

        self.ping().unwrap();

        events.push(Event::OnClientEvent(ClientEvent::OnConnected(self.remote_node_id.clone(), self.endpoint.clone())));

        (Some(self), events)
    }

}

struct SessionServer {
    local_node_id: Vec<u8>,
    remote_node_id: Vec<u8>,
    endpoint: Endpoint,
    state: SessionState,
    eph_pub: Vec<u8>,
    eph_priv: Vec<u8>,
    key_send: Vec<u8>,
    key_recv: Vec<u8>,
    cur_socket: Option<std::net::TcpStream>,
    cur_request: http::Request,

}

impl SessionServer {
    pub fn new (local_node_id: &[u8], endpoint: &Endpoint) -> error::Result<Self> {
        let (key_pub, key_priv) = libsodacrypt::kx::gen_keypair()?;
        Ok(SessionServer {
            local_node_id: local_node_id.to_vec(),
            remote_node_id: Vec::new(),
            endpoint: endpoint.clone(),
            state: SessionState::New,
            eph_pub: key_pub,
            eph_priv: key_priv,
            key_send: Vec::new(),
            key_recv: Vec::new(),
            cur_socket: None,
            cur_request: http::Request::new(http::RequestType::Request),
        })
    }

    pub fn pong (&mut self, socket: &mut std::net::TcpStream, origin_time: u64) -> error::Result<()> {
        let ping_res = message::PingRes::new(origin_time);

        let out = message::compile(
            &self.local_node_id,
            &vec![message::Message::PingRes(Box::new(ping_res))],
            http::RequestType::Response)?;

        socket.write(&out)?;

        Ok(())
    }

    fn process_initial_handshake (mut self, mut events: Vec<Event>, request: http::Request, mut socket: std::net::TcpStream) -> (Option<Self>, Vec<Event>) {
        let (mut srv_recv, mut srv_send, mut remote_node_id) = match wrap_initial_handshake(&request.path, &self.local_node_id, &self.eph_pub, &self.eph_priv, &mut socket) {
            Ok(v) => v,
            Err(e) => {
                events.push(Event::OnServerEvent(ServerEvent::OnError(error::Error::from(e))));
                return (None, events);
            }
        };

        self.remote_node_id.append(&mut remote_node_id);
        self.eph_pub.drain(..);
        self.eph_priv.drain(..);
        self.key_send.append(&mut srv_send);
        self.key_recv.append(&mut srv_recv);

        self.state = SessionState::WaitPing;

        (Some(self), events)
    }

    fn process_message (mut self, mut events: Vec<Event>, request: http::Request, mut socket: std::net::TcpStream) -> (Option<Self>, Vec<Event>) {
        let msgs = message::parse(&request.body).unwrap();
        println!("got messages: {:?}", msgs);

        for msg in msgs {
            match msg {
                message::Message::PingReq(r) => {
                    println!("got ping!: {:?}", r);
                    self.pong(&mut socket, r.sent_time);
                }
                _ => {
                    panic!("unhandled response type");
                }
            }
        }

        (Some(self), events)
    }

    pub fn process_once (mut self) -> (Option<Self>, Vec<Event>) {
        let mut buf = [0u8; 1024];
        let mut events: Vec<Event> = Vec::new();

        let mut socket = match self.cur_socket.take() {
            None => return (Some(self), events),
            Some(s) => s
        };

        if !self.cur_request.is_done() {
            let size = match socket.read(&mut buf) {
                Ok(b) => {
                    if b < 1 {
                        events.push(Event::OnServerEvent(ServerEvent::OnClose()));
                        return (Some(self), events);
                    } else {
                        b
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    self.cur_socket = Some(socket);
                    return (Some(self), events);
                }
                Err(e) => {
                    events.push(Event::OnServerEvent(ServerEvent::OnError(error::Error::from(e))));
                    events.push(Event::OnServerEvent(ServerEvent::OnClose()));
                    return (None, events);
                }
            };

            {
                if !self.cur_request.check_parse(&buf[..size]) {
                    self.cur_socket = Some(socket);
                    return (Some(self), events);
                }
            }
        }

        let request = self.cur_request;
        self.cur_request = http::Request::new(http::RequestType::Request);
        println!("GOT REQUEST: {:?}", request);

        match self.state {
            SessionState::New => {
                if self.remote_node_id.len() == 0 && &request.method == "GET" {
                    println!("initial empty get");
                    self.process_initial_handshake(events, request, socket)
                } else if self.remote_node_id.len() == 0 && &request.method == "POST" {
                    println!("empty post, need to move");
                    {
                        let parts: Vec<&str> = request.path.split('/').collect();
                        self.remote_node_id = hex::decode(parts[1]).unwrap();
                    }

                    // re-attach so we can be processed in the proper context
                    self.cur_socket = Some(socket);
                    self.cur_request = request;
                    (Some(self), events)
                } else {
                    panic!("I don't know what to do with this request!")
                }
            }
            SessionState::WaitPing => {
                if self.remote_node_id.len() == 0 {
                    panic!("cannot process non-new tx without session info");
                }
                if &request.method == "GET" {
                    panic!("cannot process GET requests on established session");
                }
                {
                    let parts: Vec<&str> = request.path.split('/').collect();
                    let remote_node_id = hex::decode(parts[1]).unwrap();
                    if remote_node_id != self.remote_node_id {
                        panic!("session id mismatch");
                    }
                }
                println!("yay we can process a session request");
                self.process_message(events, request, socket)
            }
            _ => {
                panic!("ahh, cant handle this yet: {:?}", self.state)
            }
        }
    }
}

fn wrap_connect (endpoint: &Endpoint) -> error::Result<std::net::TcpStream> {
    let timeout = std::time::Duration::from_millis(1000);
    let addr = endpoint.to_socket_addr()?;
    let socket = std::net::TcpStream::connect_timeout(&addr, timeout)?;
    socket.set_nonblocking(true)?;
    Ok(socket)
}

fn wrap_initial_connect (endpoint: &Endpoint, node_id: &[u8]) -> error::Result<SessionClient> {
    let mut socket = wrap_connect(endpoint)?;

    let mut session = SessionClient::new(node_id, endpoint)?;

    let mut req = http::Request::new(http::RequestType::Request);
    req.method = "GET".to_string();
    req.path = format!("/{}/{}",
        hex::encode(node_id),
        hex::encode(&session.eph_pub));

    socket.write(&req.generate())?;

    session.cur_socket = Some(socket);

    Ok(session)
}

fn wrap_listen (endpoint: &Endpoint) -> error::Result<std::net::TcpListener> {
    let addr = endpoint.to_socket_addr()?; 
    let socket = std::net::TcpListener::bind(addr)?;
    socket.set_nonblocking(true)?;
    Ok(socket)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InitialHandshakeRes {
    pub node_id: Vec<u8>,
    pub eph_pub: Vec<u8>,
}

fn wrap_initial_handshake (path: &str, local_node_id: &[u8], eph_pub: &[u8], eph_priv: &[u8], socket: &mut std::net::TcpStream) -> error::Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let parts: Vec<&str> = path.split('/').collect();
    let remote_node_id = hex::decode(parts[1])?;
    let cli_pub = hex::decode(parts[2])?;

    let (srv_recv, srv_send) = libsodacrypt::kx::derive_server(eph_pub, eph_priv, &cli_pub)?;

    let mut res = http::Request::new(http::RequestType::Response);
    res.status = "OK".to_string();
    res.code = "200".to_string();
    res.headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string()
    );

    let data_out = InitialHandshakeRes {
        node_id: local_node_id.to_vec(),
        eph_pub: eph_pub.to_vec(),
    };
    res.body = rmp_serde::to_vec(&data_out)?;

    socket.write(&res.generate())?;

    Ok((srv_recv, srv_send, remote_node_id))
}

fn wrap_parse_initial_handshake (data: &[u8], eph_pub: &[u8], eph_priv: &[u8]) -> error::Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let res: InitialHandshakeRes = rmp_serde::from_slice(data)?;

    let srv_pub = res.eph_pub;
    let remote_node_id = res.node_id;

    let (cli_recv, cli_send) = libsodacrypt::kx::derive_client(eph_pub, eph_priv, &srv_pub)?;

    Ok((cli_recv, cli_send, remote_node_id))
}

pub struct StdNetNode {
    node_id: Vec<u8>,
    listen_cons: Vec<StdNetListenCon>,
    server_new_cons: Vec<SessionServer>,
    server_cons: HashMap<Vec<u8>, SessionServer>,
    client_cons: Vec<SessionClient>,
    events: Vec<Event>,
}

impl StdNetNode {
    pub fn new (node_id: &[u8]) -> Self {
        StdNetNode {
            node_id: node_id.to_vec(),
            listen_cons: Vec::new(),
            server_new_cons: Vec::new(),
            server_cons: HashMap::new(),
            client_cons: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn process_once (&mut self) -> Vec<Event> {
        self.process_listen_cons();
        self.process_server_cons();
        self.process_client_cons();

        self.events.drain(..).collect()
    }

    pub fn listen (&mut self, endpoint: &Endpoint) {
        let socket = match wrap_listen(endpoint) {
            Err(e) => {
                self.events.push(Event::OnServerEvent(ServerEvent::OnError(error::Error::from(e))));
                return;
            }
            Ok(s) => s,
        };
        self.listen_cons.push(StdNetListenCon::new(socket));
        self.events.push(Event::OnServerEvent(ServerEvent::OnListening(endpoint.clone())));
    }

    pub fn connect (&mut self, endpoint: &Endpoint) {
        let session = match wrap_initial_connect(endpoint, &self.node_id) {
            Err(e) => {
                self.events.push(Event::OnClientEvent(ClientEvent::OnError(error::Error::from(e))));
                return;
            }
            Ok(s) => s,
        };
        self.client_cons.push(session);
    }

    // -- private -- //

    fn process_listen_cons (&mut self) {
        let mut new_listen_cons: Vec<StdNetListenCon> = Vec::new();
        'top: for con in self.listen_cons.drain(..) {
            loop {
                match con.socket.accept() {
                    Ok((s, addr)) => {
                        let addr = Endpoint::from(addr);
                        println!("con addr: {:?}", addr);
                        if let Err(e) = s.set_nonblocking(true) {
                            self.events.push(Event::OnServerEvent(ServerEvent::OnError(error::Error::from(e))));
                            continue;
                        }
                        let mut session = match SessionServer::new(&self.node_id, &addr) {
                            Err(e) => {
                                self.events.push(Event::OnServerEvent(ServerEvent::OnError(error::Error::from(e))));
                                continue;
                            }
                            Ok(s) => s,
                        };
                        session.cur_socket = Some(s);
                        self.server_new_cons.push(session);
                        continue;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(e) => {
                        self.events.push(Event::OnServerEvent(ServerEvent::OnError(error::Error::from(e))));
                        break 'top;
                    }
                }
            }

            new_listen_cons.push(con);
        }
        self.listen_cons = new_listen_cons;
    }

    fn process_server_cons (&mut self) {
        let mut new_cons_list: Vec<SessionServer> = Vec::new();
        let mut new_cons_hash: HashMap<Vec<u8>, SessionServer> = HashMap::new();

        for (mut _k, mut con) in self.server_cons.drain() {
            let (con, mut events) = con.process_once();
            if let Some(con) = con {
                new_cons_hash.insert(con.remote_node_id.clone(), con);
            }
            self.events.append(&mut events);
        }

        for mut con in self.server_new_cons.drain(..) {
            let (con, mut events) = con.process_once();
            if let Some(con) = con {
                if con.remote_node_id.len() > 0 {
                    let key = con.remote_node_id.clone();
                    match new_cons_hash.entry(key) {
                        hash_map::Entry::Occupied(mut e) => {
                            let session = e.get_mut();
                            println!("moving socket, dest state: {:?}", session.state);
                            session.cur_socket = con.cur_socket;
                            session.cur_request = con.cur_request;
                        }
                        hash_map::Entry::Vacant(e) => {
                            println!("vacant insert");
                            e.insert(con);
                        }
                    }
                } else {
                    new_cons_list.push(con);
                }
            }
            self.events.append(&mut events);
        }

        self.server_new_cons = new_cons_list;
        self.server_cons = new_cons_hash;
    }

    fn process_client_cons (&mut self) {
        let mut new_client_cons: Vec<SessionClient> = Vec::new();
        for mut con in self.client_cons.drain(..) {
            let (con, mut events) = con.process_once();
            if let Some(con) = con {
                new_client_cons.push(con);
            }
            self.events.append(&mut events);
        }
        self.client_cons = new_client_cons;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
