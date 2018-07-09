use error;
use net::http;
use std;
use std::io::Read;
// use std::io::{Read, Write};
// use std::net::ToSocketAddrs;

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionId (usize);

#[derive(Debug)]
pub enum ServerEvent {
    OnError(ConnectionId, error::Error),
    OnListening(ConnectionId),
    OnConnection(ConnectionId),
    OnDataReceived(ConnectionId, Vec<u8>),
    OnClose(ConnectionId),
}

#[derive(Debug)]
pub enum ClientEvent {
    OnError(ConnectionId, error::Error),
    OnConnected(ConnectionId),
    OnDataReceived(ConnectionId, Vec<u8>),
}

#[derive(Debug)]
pub enum Event {
    OnServerEvent(ServerEvent),
    OnClientEvent(ClientEvent),
}

pub trait Node {
    fn new () -> Self
    where
        Self: Sized;
    fn process_once (&mut self) -> Vec<Event>;
    fn listen (&mut self, endpoint: &Endpoint);
}

pub struct DummyNode {
    next_con_id: usize,
    events: Vec<Event>,
}

impl DummyNode {
}

impl Node for DummyNode {
    fn new () -> Self {
        DummyNode {
            next_con_id: 0,
            events: Vec::new(),
        }
    }

    fn process_once (&mut self) -> Vec<Event> {
        self.events.drain(..).collect()
    }

    fn listen (&mut self, _endpoint: &Endpoint) {
        let id = ConnectionId(self.next_con_id);
        self.next_con_id += 1;
        self.events.push(Event::OnServerEvent(ServerEvent::OnListening(id)));
    }
}

struct StdNetListenCon {
    id: ConnectionId,
    socket: std::net::TcpListener,
}

impl StdNetListenCon {
    fn new (id: ConnectionId, socket: std::net::TcpListener) -> Self {
        StdNetListenCon {
            id: id,
            socket: socket,
        }
    }
}

struct StdNetServerCon {
    id: ConnectionId,
    socket: std::net::TcpStream,
    http: http::Request,
}

impl StdNetServerCon {
    fn new (id: ConnectionId, socket: std::net::TcpStream) -> Self {
        StdNetServerCon {
            id: id,
            socket: socket,
            http: http::Request::new(http::RequestType::Request),
        }
    }
}

struct StdNetClientCon {
    id: ConnectionId,
    socket: std::net::TcpStream,
    http: http::Request,
}

impl StdNetClientCon {
    fn new (id: ConnectionId, socket: std::net::TcpStream) -> Self {
        StdNetClientCon {
            id: id,
            socket: socket,
            http: http::Request::new(http::RequestType::Response),
        }
    }
}

pub struct StdNetNode {
    next_con_id: usize,
    listen_cons: Vec<StdNetListenCon>,
    server_cons: Vec<StdNetServerCon>,
    client_cons: Vec<StdNetClientCon>,
    events: Vec<Event>,
}

impl StdNetNode {
    fn process_listen_cons (&mut self) {
        let mut new_listen_cons: Vec<StdNetListenCon> = Vec::new();
        'top: for con in self.listen_cons.drain(..) {
            loop {
                match con.socket.accept() {
                    Ok((s, _addr)) => {
                        if let Err(e) = s.set_nonblocking(true) {
                            self.events.push(Event::OnServerEvent(ServerEvent::OnError(con.id.clone(), error::Error::from(e))));
                            continue;
                        }
                        let id = ConnectionId(self.next_con_id);
                        self.next_con_id += 1;
                        self.server_cons.push(StdNetServerCon::new(id, s));
                        continue;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(e) => {
                        self.events.push(Event::OnServerEvent(ServerEvent::OnError(con.id.clone(), error::Error::from(e))));
                        break 'top;
                    }
                }
            }

            new_listen_cons.push(con);
        }
        self.listen_cons = new_listen_cons;
    }

    fn process_server_cons (&mut self) {
        let mut buf = [0u8; 1024];

        let mut new_server_cons: Vec<StdNetServerCon> = Vec::new();
        for mut con in self.server_cons.drain(..) {
            match con.socket.read(&mut buf) {
                Ok(b) => {
                    if b < 1 {
                        self.events.push(Event::OnServerEvent(ServerEvent::OnClose(con.id.clone())));
                        continue;
                    } else {
                        if con.http.check_parse(&buf[..b]) {
                            println!("GOT REQUEST: {:?}", con.http)
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
                Err(e) => {
                    self.events.push(Event::OnServerEvent(ServerEvent::OnError(con.id.clone(), error::Error::from(e))));
                    self.events.push(Event::OnServerEvent(ServerEvent::OnClose(con.id.clone())));
                    continue;
                }
            }

            new_server_cons.push(con);
        }
        self.server_cons = new_server_cons;
    }

    fn process_client_cons (&mut self) {
        let mut new_client_cons: Vec<StdNetClientCon> = Vec::new();
        'top: for con in self.client_cons.drain(..) {
            new_client_cons.push(con);
        }
        self.client_cons = new_client_cons;
    }
}

impl Node for StdNetNode {
    fn new () -> Self {
        StdNetNode {
            next_con_id: 0,
            listen_cons: Vec::new(),
            server_cons: Vec::new(),
            client_cons: Vec::new(),
            events: Vec::new(),
        }
    }

    fn process_once (&mut self) -> Vec<Event> {
        self.process_listen_cons();
        self.process_server_cons();
        self.process_client_cons();

        self.events.drain(..).collect()
    }

    fn listen (&mut self, endpoint: &Endpoint) {
        let id = ConnectionId(self.next_con_id);
        self.next_con_id += 1;
        let socket = match std::net::TcpListener::bind(format!("{}:{}", endpoint.addr, endpoint.port)) {
            Err(e) => {
                self.events.push(Event::OnServerEvent(ServerEvent::OnError(id.clone(), error::Error::from(e))));
                return;
            }
            Ok(s) => s,
        };
        socket.set_nonblocking(true).unwrap();
        self.listen_cons.push(StdNetListenCon::new(id.clone(), socket));
        self.events.push(Event::OnServerEvent(ServerEvent::OnListening(id.clone())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_exchange_data () {
        let mut node = DummyNode::new();
        node.listen(&Endpoint::new("127.0.0.1", 8080));

        assert_eq!(1, node.process_once().len());
    }
}
