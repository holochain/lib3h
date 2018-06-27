use config;
use error;
use http;
use ident;
use message;
use netinfo;
use rand;
use std;
use tcp;

static BAD_HC_PASSPHRASE: &'static [u8; 32] = &[0u8; 32];

use rand::Rng;
use tcp::{Connection, Server};

pub enum NodeEvent {
    OnWait,
    OnError(error::Error),
    OnIncomingRequest(Vec<message::Message>),
    OnOutgoingResponse(Vec<message::Message>),
}

struct NodeCon {
    socket: Box<tcp::Connection>,
    request: http::Request,
}

pub struct Node<S: tcp::Server> {
    id: ident::FullIdentity,
    this_netinfo: netinfo::NodeInfo,
    socket_srv: Option<S>,
    clients_in: Vec<Box<NodeCon>>,
    clients_out: Vec<Box<NodeCon>>,
    events: Vec<NodeEvent>,
}

impl<S> Node<S>
where
    S: tcp::Server,
{
    pub fn new(config: config::NodeConfig) -> error::Result<Node<S>> {
        let tmp: Vec<message::Message> = vec![
            message::Message::ReqNetInfoSet(Box::new(message::ReqNetInfoSet { start_tag: 42 })),
            message::Message::ReqNetInfoSet(Box::new(message::ReqNetInfoSet { start_tag: 836 })),
        ];
        let tmp = message::compile(&tmp, BAD_HC_PASSPHRASE, http::RequestType::Response)?;
        let mut tmp_req = http::Request::new(http::RequestType::Response);
        if tmp_req.check_parse(&tmp) {
            let tmp = message::parse(&tmp_req.body, BAD_HC_PASSPHRASE)?;
            println!("got parsed: {:?}", tmp);
        } else {
            println!("no parse :(");
        }

        let id = match config.identity_type {
            config::IdentityType::Ephemeral => {
                let mut rng = rand::thread_rng();
                let mut passphrase = [0u8; 16];
                rng.fill(&mut passphrase[..]);

                ident::FullIdentity::new_generate(&passphrase)?
            } // config::IdentityType::Supplied(...) => {
              //     .. some kind of file loading...
              // }
        };

        let mut node = Node {
            id: id,
            this_netinfo: netinfo::NodeInfo::new(),
            socket_srv: Some(S::new()),
            clients_in: Vec::new(),
            clients_out: Vec::new(),
            events: Vec::new(),
        };

        node.this_netinfo.id = node.id.id_hash.as_ref().unwrap().clone();
        node.this_netinfo.u32_tag = *node.id.u32_tag.as_ref().unwrap();
        node.this_netinfo.pub_keys = node.id.pub_keys.as_ref().unwrap().clone();

        // this actually needs to be discovered externally...
        // otherwise, we might get '0.0.0.0' etc...
        // or... we could add this as an explicitly set config??
        // hardcoding to loopback for now
        node.this_netinfo.endpoint = netinfo::Endpoint {
            addr: "127.0.0.1".to_string(),
            port: config.binding_endpoints[0].port,
        };

        {
            let socket_srv = match node.socket_srv.as_mut() {
                Some(v) => v,
                None => return Err(error::Error::from("no listen socket")),
            };

            // TODO - only binding to one iface for now
            //      - but you can use `0.0.0.0`
            let bind_endpoint = &config.binding_endpoints[0];
            socket_srv.listen(&bind_endpoint.addr, bind_endpoint.port);

            println!(
                "listening at {}:{}",
                &bind_endpoint.addr, bind_endpoint.port
            );

            for out_con in config.bootstrap_endpoints {
                println!("bootstrap with {:?}", out_con);
                let mut con = socket_srv.connect_out();
                con.connect(&out_con.addr, out_con.port);
                node.clients_out.push(Box::new(NodeCon {
                    socket: con,
                    request: http::Request::new(http::RequestType::Response),
                }));
            }
        }

        Ok(node)
    }

    pub fn process_once(&mut self) -> Vec<NodeEvent> {
        let mut did_something = false;
        let events;

        // first, check the server
        {
            let socket_srv = match self.socket_srv.as_mut() {
                Some(v) => v,
                None => return Vec::new(),
            };

            events = socket_srv.process_once();
        }

        for event in events {
            match event {
                tcp::Event::OnConnection(c) => {
                    did_something = true;
                    println!("GOT A CONNECTION!!");
                    let nc = Box::new(NodeCon {
                        socket: c,
                        request: http::Request::new(http::RequestType::Request),
                    });
                    self.clients_in.push(nc);
                }
                tcp::Event::OnError(e) => {
                    eprintln!("{:?}", e);
                    std::process::exit(1);
                }
                tcp::Event::OnClosed => {
                    eprintln!("server unexpectedly closed");
                    std::process::exit(1);
                }
                _ => (),
            }
        }

        // next, check incoming clients
        let mut keep_clients_in: Vec<Box<NodeCon>> = Vec::new();
        for mut nc in self.clients_in.drain(..) {
            let mut keep = true;
            let events = nc.socket.process_once();
            for event in events {
                did_something = true;
                match event {
                    tcp::Event::OnError(e) => {
                        keep = false;
                        eprintln!("{:?}", e);
                    }
                    tcp::Event::OnClosed => {
                        keep = false;
                        eprintln!("incoming client connection closed");
                    }
                    tcp::Event::OnData(data) => {
                        println!("got data {} bytes", data.len());
                        if nc.request.check_parse(&data) {
                            let msg = message::parse(&nc.request.body, BAD_HC_PASSPHRASE).unwrap();
                            self.events
                                .push(NodeEvent::OnIncomingRequest(msg));

                            let msg: Vec<message::Message> = vec![
                                message::Message::ResNetInfoSet(Box::new(message::ResNetInfoSet { net_info_set: vec![
                                    self.this_netinfo.clone()
                                ]})),
                            ];
                            let msg = message::compile(&msg, BAD_HC_PASSPHRASE, http::RequestType::Response).unwrap();
                            nc.socket.send(&msg);
                            keep = false;
                        }
                    }
                    _ => (),
                }
            }
            if keep {
                keep_clients_in.push(nc);
            }
        }
        self.clients_in = keep_clients_in;

        // next, check outgoing clients
        let mut keep_clients_out: Vec<Box<NodeCon>> = Vec::new();
        for mut nc in self.clients_out.drain(..) {
            let mut keep = true;
            let events = nc.socket.process_once();
            for event in events {
                did_something = true;
                match event {
                    tcp::Event::OnError(e) => {
                        keep = false;
                        eprintln!("{:?}", e);
                    }
                    tcp::Event::OnClosed => {
                        keep = false;
                        eprintln!("outgoing client connection closed");
                    }
                    tcp::Event::OnConnected => {
                        println!("whollly sheite, we got an outgoing connection...");
                        let msg: Vec<message::Message> = vec![
                            message::Message::ReqNetInfoSet(Box::new(message::ReqNetInfoSet { start_tag: 0 })),
                        ];
                        let msg = message::compile(&msg, BAD_HC_PASSPHRASE, http::RequestType::Request).unwrap();
                        nc.socket.send(&msg);
                    }
                    tcp::Event::OnData(data) => {
                        println!("got data {} bytes", data.len());
                        if nc.request.check_parse(&data) {
                            let msg = message::parse(&nc.request.body, BAD_HC_PASSPHRASE).unwrap();
                            self.events
                                .push(NodeEvent::OnOutgoingResponse(msg));

                            keep = false;
                        }
                    }
                    _ => (),
                }
            }
            if keep {
                keep_clients_out.push(nc);
            }
        }
        self.clients_out = keep_clients_out;

        if self.events.len() > 0 {
            self.events.drain(..).collect()
        } else if did_something {
            vec![NodeEvent::OnWait]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static PASSPHRASE: &'static [u8] = b"this is a test passphrase";

    #[test]
    fn it_listens() {
        /*
        let id = ident::FullIdentity::new_generate(PASSPHRASE).unwrap();
        let bind_endpoint = Endpoint {
            addr: String::from("127.0.0.1"),
            port: 12345,
        };
        let mut node: Node<tcp::DummyConnection, tcp::DummyServer> =
            Node::new(id, bind_endpoint, Vec::new()).unwrap();

        {
            let con = Box::new(tcp::DummyConnection::new());
            node.socket_srv.as_mut().unwrap().sim_connection(con);
        }

        node.process_once();
        */
    }
}
