use error;
use std;
use std::io::{Read, Write};
use std::net::ToSocketAddrs;

pub enum Event {
    OnError(error::Error),
    OnConnected,
    OnConnection(Box<Connection>),
    OnData(Vec<u8>),
    OnClosed,
}

// -- connection -- //

pub trait Connection {
    fn new() -> Self
    where
        Self: Sized;
    fn is_active(&self) -> bool;
    fn connect(&mut self, addr: &str, port: u16);
    fn send(&mut self, data: &[u8]);
    fn process_once(&mut self) -> Vec<Event>;
}

pub struct DummyConnection {
    is_active: bool,
    events: Vec<Event>,
}

impl DummyConnection {
    // -- utilities for simulating activity -- //

    #[allow(dead_code)]
    pub fn sim_error(&mut self, e: error::Error) {
        self.events.push(Event::OnError(e));
        self.events.push(Event::OnClosed);
        self.is_active = false;
    }

    #[allow(dead_code)]
    pub fn sim_close(&mut self) {
        self.events.push(Event::OnClosed);
        self.is_active = false;
    }

    #[allow(dead_code)]
    pub fn sim_recv(&mut self, data: &[u8]) {
        self.events.push(Event::OnData(data.to_vec()));
    }
}

impl Connection for DummyConnection {
    fn new() -> Self {
        DummyConnection {
            is_active: true,
            events: Vec::new(),
        }
    }

    fn is_active(&self) -> bool {
        self.is_active
    }

    fn connect(&mut self, _addr: &str, _port: u16) {
        self.events.push(Event::OnConnected);
    }

    fn send(&mut self, _data: &[u8]) {}

    fn process_once(&mut self) -> Vec<Event> {
        self.events.drain(..).collect()
    }
}

pub struct StdNetConnection {
    socket: Option<std::net::TcpStream>,
    events: Vec<Event>,
}

impl StdNetConnection {
    pub fn new_from_tcp_stream(socket: std::net::TcpStream) -> Self {
        StdNetConnection {
            socket: Some(socket),
            events: Vec::new(),
        }
    }
}

fn try_connect(addr: &str, port: u16) -> error::Result<std::net::TcpStream> {
    let timeout = std::time::Duration::from_millis(1000);
    let mut addr = format!("{}:{}", addr, port).to_socket_addrs()?;
    let addr = match addr.next() {
        Some(v) => v,
        None => return Err(error::Error::from("socket addr gen failure")),
    };

    let socket = std::net::TcpStream::connect_timeout(&addr, timeout)?;
    socket.set_nonblocking(true)?;
    Ok(socket)
}

impl Connection for StdNetConnection {
    fn new() -> Self {
        StdNetConnection {
            socket: None,
            events: Vec::new(),
        }
    }

    fn is_active(&self) -> bool {
        match self.socket {
            Some(_) => true,
            None => false,
        }
    }

    fn connect(&mut self, addr: &str, port: u16) {
        self.socket = Some(match try_connect(addr, port) {
            Ok(v) => v,
            Err(e) => {
                self.events.push(Event::OnError(e));
                return;
            }
        });

        self.events.push(Event::OnConnected);
    }

    fn send(&mut self, data: &[u8]) {
        self.socket.as_mut().unwrap().write(data).unwrap();
    }

    fn process_once(&mut self) -> Vec<Event> {
        if let None = self.socket {
            return self.events.drain(..).collect();
        }

        let mut buf = [0u8; 1024];

        match self.socket.as_mut().unwrap().read(&mut buf) {
            Ok(b) => {
                if b < 1 {
                    self.socket = None;
                    self.events.push(Event::OnClosed);
                } else {
                    self.events.push(Event::OnData(buf[..b].to_vec()));
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
            Err(e) => {
                self.socket = None;
                self.events
                    .push(Event::OnError(error::Error::generic_error(Box::new(e))));
                self.events.push(Event::OnClosed);
            }
        };

        self.events.drain(..).collect()
    }
}

// -- server -- //

pub trait Server {
    fn new() -> Self
    where
        Self: Sized;
    fn is_active(&self) -> bool;
    fn listen(&mut self, addr: &str, port: u16);
    fn process_once(&mut self) -> Vec<Event>;
    fn connect_out(&self) -> Box<Connection>;
}

pub struct DummyServer {
    is_active: bool,
    events: Vec<Event>,
}

impl DummyServer {
    // -- utilities for simulating activity -- //

    #[allow(dead_code)]
    pub fn sim_error(&mut self, e: error::Error) {
        self.events.push(Event::OnError(e));
        self.is_active = false;
    }

    #[allow(dead_code)]
    pub fn sim_close(&mut self) {
        self.events.push(Event::OnClosed);
        self.is_active = false;
    }

    #[allow(dead_code)]
    pub fn sim_connection(&mut self, con: Box<Connection>) {
        self.events.push(Event::OnConnection(con));
    }
}

impl Server for DummyServer {
    fn new() -> Self {
        DummyServer {
            is_active: true,
            events: Vec::new(),
        }
    }

    fn is_active(&self) -> bool {
        self.is_active
    }

    fn listen(&mut self, _addr: &str, _port: u16) {
        self.events.push(Event::OnConnected);
    }

    fn process_once(&mut self) -> Vec<Event> {
        self.events.drain(..).collect()
    }

    fn connect_out(&self) -> Box<Connection> {
        Box::new(DummyConnection::new())
    }
}

pub struct StdNetServer {
    socket: Option<std::net::TcpListener>,
    events: Vec<Event>,
}

impl Server for StdNetServer {
    fn new() -> Self {
        StdNetServer {
            socket: None,
            events: Vec::new(),
        }
    }

    fn is_active(&self) -> bool {
        match self.socket {
            Some(_) => true,
            None => false,
        }
    }

    fn listen(&mut self, addr: &str, port: u16) {
        let socket = std::net::TcpListener::bind(format!("{}:{}", addr, port)).unwrap();
        socket.set_nonblocking(true).unwrap();
        self.socket = Some(socket);

        self.events.push(Event::OnConnected);
    }

    fn process_once(&mut self) -> Vec<Event> {
        loop {
            let r = self.socket.as_mut().unwrap().accept();
            match r {
                Ok((s, _addr)) => {
                    s.set_nonblocking(true).unwrap();
                    let con = Box::new(StdNetConnection::new_from_tcp_stream(s));
                    self.events.push(Event::OnConnection(con));
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    self.socket = None;
                    self.events
                        .push(Event::OnError(error::Error::generic_error(Box::new(e))));
                    self.events.push(Event::OnClosed);
                }
            }
        }

        self.events.drain(..).collect()
    }

    fn connect_out(&self) -> Box<Connection> {
        Box::new(StdNetConnection::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_connection_full() {
        let mut con = DummyConnection::new();

        con.connect("127.0.0.1", 8080);

        'outer: loop {
            let events = con.process_once();
            if events.len() < 1 {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            for event in events {
                match event {
                    Event::OnError(e) => panic!(format!("{}", e)),
                    Event::OnConnected => {
                        let tmp = [0u8, 1u8, 2u8];
                        con.send(&tmp);
                        con.sim_recv(&tmp);
                    }
                    Event::OnConnection(_) => {
                        panic!("this happens on a server, not a connection");
                    }
                    Event::OnData(_d) => {
                        con.sim_close();
                    }
                    Event::OnClosed => {
                        break 'outer;
                    }
                };
            }
        }
    }

    #[test]
    fn dummy_connection_error() {
        let mut con = DummyConnection::new();
        con.sim_error(error::Error::from("test"));

        'outer: loop {
            let events = con.process_once();
            for event in events {
                match event {
                    Event::OnError(_e) => {
                        assert_eq!(con.is_active(), false);
                        break 'outer;
                    }
                    _ => (),
                }
            }
        }
    }

    #[test]
    fn dummy_server_full() {
        let mut con = DummyServer::new();

        con.listen("127.0.0.1", 8080);

        'outer: loop {
            let events = con.process_once();
            if events.len() < 1 {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            for event in events {
                match event {
                    Event::OnError(e) => panic!(format!("{}", e)),
                    Event::OnConnected => {
                        con.sim_connection(Box::new(DummyConnection::new()));
                    }
                    Event::OnConnection(_) => {
                        con.sim_close();
                    }
                    Event::OnData(_d) => {
                        panic!("this happens on a connection, not a server");
                    }
                    Event::OnClosed => {
                        break 'outer;
                    }
                };
            }
        }
    }

    #[test]
    fn dummy_server_error() {
        let mut con = DummyServer::new();
        con.sim_error(error::Error::from("test"));

        'outer: loop {
            let events = con.process_once();
            for event in events {
                match event {
                    Event::OnError(_e) => {
                        assert_eq!(con.is_active(), false);
                        break 'outer;
                    }
                    _ => (),
                }
            }
        }
    }
}
