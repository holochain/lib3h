use crate::transport::error::{TransportError, TransportResult};
use lib3h_protocol::DidWork;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, RwLock},
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
    ReceivedData(Url, Vec<u8>),
    /// A connection closed for whatever reason
    ConnectionClosed(Url),
}

//--------------------------------------------------------------------------------------------------
// Memory Server MAP
//--------------------------------------------------------------------------------------------------

/// Type for holding a map of 'url -> InMemoryServer'
type MemoryServerMap = HashMap<Url, Mutex<MemoryServer>>;

// this is the actual memory space for our in-memory servers
lazy_static! {
    pub(crate) static ref MEMORY_SERVER_MAP: RwLock<MemoryServerMap> = RwLock::new(HashMap::new());
    static ref URL_COUNT: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
}

pub fn new_url() -> Url {
    let mut tc = URL_COUNT
        .lock()
        .expect("could not lock transport count mutex");
    *tc += 1;
    Url::parse(&format!("mem://addr_{}", *tc).as_str()).unwrap()
}

/// Add new MemoryServer to the global server map
pub fn set_server(uri: &Url) -> TransportResult<()> {
    debug!("MemoryServer::set_server: {}", uri);
    // Create server with that name if it doesn't already exist
    let mut server_map = MEMORY_SERVER_MAP.write().unwrap();
    if server_map.contains_key(uri) {
        return Ok(());
    }
    let server = MemoryServer::new(uri);
    server_map.insert(uri.clone(), Mutex::new(server));
    Ok(())
}

/// Remove a MemoryServer from the global server map
pub fn unset_server(uri: &Url) -> TransportResult<()> {
    debug!("MemoryServer::unset_server: {}", uri);
    // Create server with that name if it doesn't already exist
    let mut server_map = MEMORY_SERVER_MAP.write().unwrap();
    if !server_map.contains_key(uri) {
        return Err(TransportError::new("Server doesn't exist".to_string()));
    }
    server_map.remove(uri);
    Ok(())
}

//--------------------------------------------------------------------------------------------------
// Memory Server
//--------------------------------------------------------------------------------------------------

/// We use the uri as the connectionId
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
                    "(MemoryServer {}) received: {} (from {})",
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
