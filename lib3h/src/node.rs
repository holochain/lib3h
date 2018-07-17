pub use libsodacon::net::endpoint::Endpoint;

use libsodacon::net::event::{ClientEvent, Event as SCEvent, ServerEvent};
use libsodacon::node::StdNetNode;

use message;

use std;
use std::collections::HashMap;

use errors::*;

#[derive(Debug)]
pub enum Event {
    OnError(Error),
    OnReady,
    OnData(Vec<u8>, Vec<u8>),
}

pub struct Node {
    sodacon: StdNetNode,
    events: Vec<Event>,

    // did our initial listen / connection list get satisfied?
    published_ready: bool,

    // track our initial listen / connection list
    wait_listening: Vec<Endpoint>,
    wait_connecting: Vec<Endpoint>,

    // have we ever tried to connect to this node?
    // need to keep timing stuff in the future.
    tried_connect: Vec<Endpoint>,

    // last discovery check
    last_discovery: std::time::Instant,
}

impl Node {
    pub fn new(
        node_id: &[u8],
        listen_endpoints: &[Endpoint],
        discovery_endpoints: &[Endpoint],
        bootstrap_connections: &[Endpoint],
    ) -> Self {
        let mut node = StdNetNode::new(node_id);
        for discover_endpoint in discovery_endpoints {
            node.add_local_discover_endpoint(discover_endpoint);
        }

        let mut tried_connect: Vec<Endpoint> = Vec::new();

        let mut wait_listening: Vec<Endpoint> = Vec::new();
        let mut wait_connecting: Vec<Endpoint> = Vec::new();

        for listen_endpoint in listen_endpoints {
            tried_connect.push(listen_endpoint.clone());
            node.listen(listen_endpoint);
            wait_listening.push(listen_endpoint.clone());
        }

        for bootstrap_connection in bootstrap_connections {
            tried_connect.push(bootstrap_connection.clone());
            node.connect(bootstrap_connection);
            wait_connecting.push(bootstrap_connection.clone());
        }

        Node {
            sodacon: node,
            events: Vec::new(),
            published_ready: false,
            wait_listening,
            wait_connecting,
            tried_connect,
            last_discovery: std::time::Instant::now(),
        }
    }

    pub fn get_node_id(&self) -> Vec<u8> {
        self.sodacon.get_node_id()
    }

    pub fn list_connected_nodes(&self) -> Vec<Vec<u8>> {
        self.sodacon.list_connected_nodes()
    }

    pub fn send(&mut self, dest_node_id: &[u8], data: Vec<u8>) {
        let data = message::compile(&message::Message::UserMessage(Box::new(
            message::UserMessage::new(data),
        ))).unwrap();
        self.sodacon.send(dest_node_id, data);
    }

    pub fn process_once(&mut self) -> Vec<Event> {
        // -- check for new discoverable nodes -- //

        if self.last_discovery.elapsed() > std::time::Duration::from_millis(2000) {
            self.publish_discovery_request();
        }

        // -- process events -- //

        let events;
        {
            events = self.sodacon.process_once();
        }

        for event in events {
            match event {
                SCEvent::OnError(e) => {
                    self.events.push(Event::OnError(e.into()));
                }
                SCEvent::OnServerEvent(ev) => match ev {
                    ServerEvent::OnListening(endpoint) => {
                        self.wait_listening.retain(|ref ep| {
                            return *ep != &endpoint;
                        });
                    }
                    ServerEvent::OnDataReceived(node_id, data) => {
                        self.handle_incoming_message(node_id, &data);
                    }
                    _ => (),
                },
                SCEvent::OnClientEvent(ev) => match ev {
                    ClientEvent::OnConnected(_node_id, endpoint) => {
                        self.wait_connecting.retain(|ref ep| {
                            return *ep != &endpoint;
                        });
                    }
                    ClientEvent::OnDataReceived(node_id, data) => {
                        self.handle_incoming_message(node_id, &data);
                    }
                    _ => (),
                },
            }
        }

        if !self.published_ready
            && self.wait_listening.is_empty()
            && self.wait_connecting.is_empty()
        {
            self.published_ready = true;
            self.events.push(Event::OnReady);

            self.publish_discovery_request();
        }

        self.events.drain(..).collect()
    }

    // -- private -- //

    fn publish_discovery_request(&mut self) {
        let discovery_map = self.sodacon.list_discoverable();

        for node_id in self.sodacon.list_connected_nodes() {
            let data = message::compile(&message::Message::DiscoveryReq(Box::new(
                message::DiscoveryReq::new(discovery_map.clone()),
            ))).unwrap();
            self.sodacon.send(&node_id, data);
        }

        self.last_discovery = std::time::Instant::now();
    }

    fn publish_discovery_response(&mut self, node_id: &[u8]) {
        let discovery_map = self.sodacon.list_discoverable();

        let data = message::compile(&message::Message::DiscoveryRes(Box::new(
            message::DiscoveryRes::new(discovery_map),
        ))).unwrap();
        self.sodacon.send(&node_id, data);
    }

    fn handle_discovery(&mut self, d: HashMap<Vec<u8>, Vec<Endpoint>>) {
        let connected = self.sodacon.list_connected_nodes();
        'top: for (node_id, discover_item) in d {
            for c_node_id in &connected {
                if c_node_id == &node_id {
                    continue 'top;
                }
            }
            for c_endpoint in &self.tried_connect {
                if c_endpoint == &discover_item[0] {
                    continue 'top;
                }
            }

            println!(
                "Discovered Connection to [{}]: {:?}",
                String::from_utf8_lossy(&node_id),
                &discover_item[0]
            );
            self.tried_connect.push(discover_item[0].clone());
            self.sodacon.connect(&discover_item[0]);
        }
    }

    fn handle_incoming_message(&mut self, node_id: Vec<u8>, data: &[u8]) {
        match message::parse(&data).unwrap() {
            message::Message::UserMessage(m) => {
                self.events.push(Event::OnData(node_id, m.data));
            }
            message::Message::DiscoveryReq(r) => {
                self.publish_discovery_response(&node_id);
                self.handle_discovery(r.discover);
            }
            message::Message::DiscoveryRes(r) => {
                self.handle_discovery(r.discover);
            }
        }
    }
}
