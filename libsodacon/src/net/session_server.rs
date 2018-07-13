use error;
use hex;
use libsodacrypt;
use net::endpoint::Endpoint;
use net::event::{Event, ServerEvent};
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

pub struct SessionServer {
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
    pub cur_request: http::Request,
}

impl SessionServer {
    pub fn new (local_node_id: &[u8], endpoint: &Endpoint) -> error::Result<Self> {
        let (key_pub, key_priv) = libsodacrypt::kx::gen_keypair()?;
        Ok(SessionServer {
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
            cur_request: http::Request::new(http::RequestType::Request),
        })
    }

    pub fn pong (&mut self, socket: &mut std::net::TcpStream, origin_time: u64) -> error::Result<()> {
        let ping_res = message::PingRes::new(origin_time);

        let out = message::compile(
            &self.session_id,
            &vec![message::Message::PingRes(Box::new(ping_res))],
            http::RequestType::Response,
            &self.key_send)?;

        socket.write(&out)?;

        Ok(())
    }

    fn process_initial_handshake (mut self, mut events: Vec<Event>, request: http::Request, mut socket: std::net::TcpStream) -> (Option<Self>, Vec<Event>) {
        let (mut srv_recv, mut srv_send, mut remote_node_id, session_id) = match wrap_initial_handshake(&request.path, &self.local_node_id, &self.eph_pub, &self.eph_priv, &mut socket) {
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
        self.session_id = session_id;

        self.state = SessionState::WaitPing;

        (Some(self), events)
    }

    fn process_message (mut self, events: Vec<Event>, request: http::Request, mut socket: std::net::TcpStream) -> (Option<Self>, Vec<Event>) {
        let msgs = message::parse(&request.body, &self.key_recv).unwrap();
        println!("got messages: {:?}", msgs);

        for msg in msgs {
            match msg {
                message::Message::PingReq(r) => {
                    println!("got ping!: {:?}", r);
                    self.state = SessionState::Ready;
                    self.pong(&mut socket, r.sent_time).unwrap();
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
                if self.session_id.len() == 0 && &request.method == "GET" {
                    self.process_initial_handshake(events, request, socket)
                } else if self.session_id.len() == 0 && &request.method == "POST" {
                    {
                        let parts: Vec<&str> = request.path.split('/').collect();
                        self.session_id = parts[1].to_string();
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
                if self.session_id.len() == 0 {
                    panic!("cannot process non-new tx without session info");
                }
                if &request.method == "GET" {
                    panic!("cannot process GET requests on established session");
                }
                {
                    let parts: Vec<&str> = request.path.split('/').collect();
                    if parts[1] != self.session_id {
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

fn wrap_initial_handshake (path: &str, local_node_id: &[u8], eph_pub: &[u8], eph_priv: &[u8], socket: &mut std::net::TcpStream) -> error::Result<(Vec<u8>, Vec<u8>, Vec<u8>, String)> {
    let parts: Vec<&str> = path.split('/').collect();
    let remote_node_id = hex::decode(parts[1])?;
    let cli_pub = hex::decode(parts[2])?;

    let session_id = hex::encode(libsodacrypt::rand::rand_bytes(32)?);

    let (srv_recv, srv_send) = libsodacrypt::kx::derive_server(eph_pub, eph_priv, &cli_pub)?;

    let mut res = http::Request::new(http::RequestType::Response);
    res.status = "OK".to_string();
    res.code = "200".to_string();
    res.headers.insert(
        "content-type".to_string(),
        "application/octet-stream".to_string()
    );

    let data_out = message::InitialHandshakeRes {
        session_id: session_id.clone(),
        node_id: local_node_id.to_vec(),
        eph_pub: eph_pub.to_vec(),
    };
    res.body = rmp_serde::to_vec(&data_out)?;

    socket.write(&res.generate())?;

    Ok((srv_recv, srv_send, remote_node_id, session_id))
}
