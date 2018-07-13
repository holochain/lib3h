pub use libsodacon::net::endpoint::Endpoint;

use libsodacon::net::event::{Event as SCEvent, ServerEvent, ClientEvent};
use libsodacon::node::StdNetNode;

use error;

#[derive(Debug)]
pub enum Event {
    OnError(error::Error),
    OnReady,
    OnData(Vec<u8>, Vec<u8>),
}

pub struct Node {
    sodacon: StdNetNode,
    events: Vec<Event>,
    published_ready: bool,
    wait_listening: Vec<Endpoint>,
    wait_connecting: Vec<Endpoint>,
}

impl Node {
    pub fn new (
        node_id: &[u8],
        listen_endpoints: &[Endpoint],
        bootstrap_connections: &[Endpoint],
    ) -> Self {
        let mut node = StdNetNode::new(node_id);
        let mut wait_listening: Vec<Endpoint> = Vec::new();
        let mut wait_connecting: Vec<Endpoint> = Vec::new();

        for listen_endpoint in listen_endpoints {
            node.listen(listen_endpoint);
            wait_listening.push(listen_endpoint.clone());
        }

        for bootstrap_connection in bootstrap_connections {
            node.connect(bootstrap_connection);
            wait_connecting.push(bootstrap_connection.clone());
        }

        Node {
            sodacon: node,
            events: Vec::new(),
            published_ready: false,
            wait_listening: wait_listening,
            wait_connecting: wait_connecting,
        }
    }

    pub fn get_node_id (&self) -> Vec<u8> {
        self.sodacon.get_node_id()
    }

    pub fn get_connected_nodes (&self) -> Vec<Vec<u8>> {
        self.sodacon.get_connected_nodes()
    }

    pub fn send (&mut self, dest_node_id: &[u8], data: &[u8]) {
        self.sodacon.send(dest_node_id, data);
    }

    pub fn process_once (&mut self) -> Vec<Event> {
        let events;
        {
            events = self.sodacon.process_once();
        }

        for event in events {
            match event {
                SCEvent::OnError(e) => {
                    self.events.push(Event::OnError(error::Error::from(e)));
                }
                SCEvent::OnServerEvent(ev) => {
                    match ev {
                        ServerEvent::OnListening(endpoint) => {
                            self.wait_listening.retain(|ref ep| {
                                return *ep != &endpoint;
                            });
                        }
                        ServerEvent::OnDataReceived(node_id, message) => {
                            self.events.push(Event::OnData(node_id, message));
                        }
                        _ => (),
                    }
                }
                SCEvent::OnClientEvent(ev) => {
                    match ev {
                        ClientEvent::OnConnected(node_id, endpoint) => {
                            self.wait_connecting.retain(|ref ep| {
                                return *ep != &endpoint;
                            });
                        }
                        _ => (),
                    }
                }
            }
        }

        if !self.published_ready && self.wait_listening.len() == 0 && self.wait_connecting.len() == 0 {
            self.published_ready = true;
            self.events.push(Event::OnReady);
        }

        self.events.drain(..).collect()
    }
}
