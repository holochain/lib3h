use error;
use hex;
use libsodacrypt;
use net::endpoint::Endpoint;
use net::event::{Event, ClientEvent};
use net::message;
use net::http;
use rmp_serde;
use std;
use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    New,
    WaitPing,
    Ready,
}

pub struct SessionClient {
    pub session_id: String,
    pub local_node_id: Vec<u8>,
    pub remote_node_id: Vec<u8>,
    pub endpoint: Endpoint,
    pub state: SessionState,
    pub eph_pub: Vec<u8>,
    pub eph_priv: Vec<u8>,
    pub key_send: Vec<u8>,
    pub key_recv: Vec<u8>,
    pub cur_socket: Option<std::net::TcpStream>,
    pub cur_response: http::Request,
}

impl SessionClient {
    pub fn new (local_node_id: &[u8], endpoint: &Endpoint) -> error::Result<Self> {
        let (key_pub, key_priv) = libsodacrypt::kx::gen_keypair()?;
        Ok(SessionClient {
            session_id: "".to_string(),
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

    pub fn new_initial_connect (endpoint: &Endpoint, node_id: &[u8]) -> error::Result<Self> {
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

    pub fn ping (&mut self) -> error::Result<()> {
        let mut socket = wrap_connect(&self.endpoint)?;

        let ping_req = message::PingReq::new();

        let out = message::compile(
            &self.session_id,
            &vec![message::Message::PingReq(Box::new(ping_req))],
            http::RequestType::Request,
            &self.key_send)?;

        socket.write(&out)?;

        self.cur_socket = Some(socket);

        Ok(())
    }

    pub fn user_message (&mut self, data: &[u8]) -> error::Result<()> {
        let mut socket = wrap_connect(&self.endpoint)?;

        let msg = message::UserMessage::new(data);

        let out = message::compile(
            &self.session_id,
            &vec![message::Message::UserMessage(Box::new(msg))],
            http::RequestType::Request,
            &self.key_send)?;

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

        match self.state {
            SessionState::New => {
                self.process_initial_handshake(events, response)
            }
            SessionState::WaitPing => {
                self.process_initial_ping(events, response)
            }
            _ => {
                panic!("ahh, cant handle this yet: {:?}", self.state);
            }
        }
    }

    // -- private -- //

    fn process_initial_handshake (mut self, mut events: Vec<Event>, response: http::Request) -> (Option<Self>, Vec<Event>) {
        let (mut cli_recv, mut cli_send, mut remote_node_id, session_id) = match wrap_parse_initial_handshake(&response.body, &self.eph_pub, &self.eph_priv) {
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
        self.session_id = session_id;

        self.state = SessionState::WaitPing;

        self.ping().unwrap();

        (Some(self), events)
    }

    fn process_initial_ping (mut self, mut events: Vec<Event>, response: http::Request) -> (Option<Self>, Vec<Event>) {
        let msg = message::parse(&response.body, &self.key_recv).unwrap();
        match msg[0] {
            message::Message::PingRes(ref r) => {
                println!("ping response in {} ms",
                    message::get_millis() - r.origin_time);
                events.push(Event::OnClientEvent(ClientEvent::OnConnected(self.remote_node_id.clone(), self.endpoint.clone())));

            }
            _ => {
                panic!("unexpected message: {:?}", msg);
            }
        }

        self.state = SessionState::Ready;

        (Some(self), events)
    }
}

fn wrap_connect (endpoint: &Endpoint) -> error::Result<std::net::TcpStream> {
    let timeout = std::time::Duration::from_millis(1000);
    let addr = endpoint.to_socket_addr()?;
    let socket = std::net::TcpStream::connect_timeout(&addr, timeout)?;
    socket.set_nonblocking(true)?;
    Ok(socket)
}

fn wrap_parse_initial_handshake (data: &[u8], eph_pub: &[u8], eph_priv: &[u8]) -> error::Result<(Vec<u8>, Vec<u8>, Vec<u8>, String)> {
    let res: message::InitialHandshakeRes = rmp_serde::from_slice(data)?;

    let srv_pub = res.eph_pub;
    let remote_node_id = res.node_id;
    let session_id = res.session_id;

    let (cli_recv, cli_send) = libsodacrypt::kx::derive_client(eph_pub, eph_priv, &srv_pub)?;

    Ok((cli_recv, cli_send, remote_node_id, session_id))
}
