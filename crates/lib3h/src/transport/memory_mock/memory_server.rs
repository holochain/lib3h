use crate::transport::error::{TransportError, TransportResult};
use lib3h_protocol::{data_types::Opaque, Address, DidWork};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Mutex, MutexGuard},
};
use url::Url;

//--------------------------------------------------------------------------------------------------
// Memory Server protocol
//--------------------------------------------------------------------------------------------------

/// Events that can be generated during a `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum MemoryEvent {
    /// we have received an incoming connection
    IncomingConnectionEstablished(Url),
    /// We have received data from a connection
    ReceivedData(Url, Opaque),
    /// A connection closed for whatever reason
    ConnectionClosed(Url),
}

//--------------------------------------------------------------------------------------------------
// Memory Server MAP
//--------------------------------------------------------------------------------------------------

/// Type for holding a map of 'url -> InMemoryServer'
pub struct MemoryNet {
    pub server_map: HashMap<Url, MemoryServer>,
    url_count: u32,
    advertised_machines: HashSet<(Url, Address)>,
}

impl MemoryNet {
    pub fn new() -> Self {
        MemoryNet {
            server_map: HashMap::new(),
            url_count: 0,
            advertised_machines: HashSet::new(),
        }
    }
    pub fn advertise(&mut self, uri: Url, machine_id: Address) {
        self.advertised_machines.insert((uri, machine_id));
    }
    pub fn discover(&mut self) -> Vec<(Url, Address)> {
        self.advertised_machines.iter().cloned().collect()
    }
    pub fn new_url(&mut self) -> Url {
        self.url_count += 1;
        Url::parse(&format!("mem://addr_{}", self.url_count).as_str()).unwrap()
    }
}

/// Holds a universe of memory networks so we can run tests in separate universes
pub struct MemoryVerse {
    server_maps: HashMap<String, MemoryNet>,
}
impl MemoryVerse {
    pub fn new() -> Self {
        MemoryVerse {
            server_maps: HashMap::new(),
        }
    }
    pub fn get_net(&mut self, network: &str) -> Option<&mut MemoryNet> {
        self.server_maps.get_mut(network)
    }

    pub fn get_server(&mut self, network: &str, url: &Url) -> Option<&mut MemoryServer> {
        self.server_maps.get_mut(network)?.server_map.get_mut(url)
    }

    pub fn bind(&mut self, network: &str) -> Url {
        let net = self
            .server_maps
            .entry(network.to_string())
            .or_insert_with(|| MemoryNet::new());
        let binding = net.new_url();
        net.server_map
            .entry(binding.clone())
            .or_insert_with(|| MemoryServer::new(&binding));
        binding
    }
}

// this is the actual memory space for our in-memory servers
lazy_static! {
    pub static ref MEMORY_VERSE: Mutex<MemoryVerse> = Mutex::new(MemoryVerse::new());
}

pub fn get_memory_verse<'a>() -> MutexGuard<'a, MemoryVerse> {
    for _ in 0..100 {
        match MEMORY_VERSE.try_lock() {
            Ok(l) => return l,
            _ => std::thread::sleep(std::time::Duration::from_millis(1)),
        }
    }
    panic!("unable to obtain mutex lock on MEMORY_VERSE");
}

//--------------------------------------------------------------------------------------------------
// Memory Server
//--------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct MemoryServer {
    /// Address of this server
    this_uri: Url,
    /// Inboxes for payloads from each of its connections.
    inbox_map: HashMap<Url, VecDeque<Vec<u8>>>,
    /// Inbox of connection state change requests
    /// (true = incoming connection, false = connection closed)
    connection_inbox: Vec<(Url, bool)>,
}

impl Drop for MemoryServer {
    fn drop(&mut self) {
        trace!("(MemoryServer) dropped: {:?}", self.this_uri);
    }
}

impl MemoryServer {
    /// Constructor
    pub fn new(uri: &Url) -> Self {
        MemoryServer {
            this_uri: uri.clone(),
            inbox_map: HashMap::new(),
            connection_inbox: Vec::new(),
        }
    }

    pub fn is_connected_to(&self, uri: &Url) -> bool {
        self.inbox_map.contains_key(uri)
    }

    /// Another node requested to connect with us.
    /// This creates a new connection: An inbox is created for receiving payloads from this requester.
    /// This also generates a request for us to connect to the other node in the other way.
    pub fn request_connect(&mut self, other_uri: &Url) -> TransportResult<()> {
        info!(
            "(MemoryServer) {} creates inbox for {}",
            self.this_uri, other_uri
        );
        if other_uri == &self.this_uri {
            return Err(TransportError::new(format!(
                "Server {} cannot connect to self",
                self.this_uri,
            )));
        }
        if self.inbox_map.contains_key(other_uri) {
            return Err(TransportError::new(format!(
                "Server {}, is already connected to {}",
                self.this_uri, other_uri,
            )));
        }
        // Establish connection
        let prev = self.inbox_map.insert(other_uri.clone(), VecDeque::new());
        assert!(prev.is_none());
        // Notify our TransportMemory (so it can connect back)
        self.connection_inbox.push((other_uri.clone(), true));
        // Done
        Ok(())
    }

    /// Another node closes its connection with us
    pub fn request_close(&mut self, other_uri: &Url) -> TransportResult<()> {
        info!("(MemoryServer {}).close({})", self.this_uri, other_uri);
        // delete this uri's inbox
        let res = self.inbox_map.remove(other_uri);
        if res.is_none() {
            return Err(TransportError::new(format!(
                "uri '{}' unknown for server {}",
                other_uri, self.this_uri
            )));
        }
        trace!("(MemoryServer {}). close event", self.this_uri);
        // Notify our TransportMemory
        self.connection_inbox.push((other_uri.clone(), false));
        // Done
        Ok(())
    }

    /// Receive payload from another node, i.e. fill our inbox for that uri
    pub fn post(&mut self, from_uri: &Url, payload: &[u8]) -> TransportResult<()> {
        let maybe_inbox = self.inbox_map.get_mut(from_uri);
        if let None = maybe_inbox {
            return Err(TransportError::new(format!(
                "(MemoryServer {}) Unknown from_uri {}",
                self.this_uri, from_uri
            )));
        }
        maybe_inbox.unwrap().push_back(payload.to_vec());
        Ok(())
    }

    /// Process all inboxes: payload inboxes and incoming connections inbox.
    /// Return a TransportEvent::ReceivedData for each payload processed and
    /// a TransportEvent::IncomingConnectionEstablished for each incoming connection.
    pub fn process(&mut self) -> TransportResult<(DidWork, Vec<MemoryEvent>)> {
        trace!("(MemoryServer {}).process()", self.this_uri);
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Process connection inbox
        for (in_uri, is_new) in self.connection_inbox.iter() {
            trace!(
                "(MemoryServer {}). connection_inbox: {} | {}",
                self.this_uri,
                in_uri,
                is_new,
            );
            let event = if *is_new {
                MemoryEvent::IncomingConnectionEstablished(in_uri.clone())
            } else {
                MemoryEvent::ConnectionClosed(in_uri.clone())
            };
            trace!("(MemoryServer {}). connection: {:?}", self.this_uri, event);
            outbox.push(event);
            did_work = true;
        }
        self.connection_inbox.clear();
        // Process msg inboxes
        for (uri, inbox) in self.inbox_map.iter_mut() {
            loop {
                let payload = match inbox.pop_front() {
                    None => break,
                    Some(msg) => msg,
                };
                did_work = true;
                trace!(
                    "(MemoryServer {}) received: {}B (from {})",
                    self.this_uri,
                    payload.len(),
                    uri
                );
                let evt = MemoryEvent::ReceivedData(uri.clone(), payload.into());
                outbox.push(evt);
            }
        }
        Ok((did_work, outbox))
    }
}
