use crate::transport::{
    error::{TransportError, TransportResult},
    protocol::TransportEvent,
    ConnectionId, ConnectionIdRef,
};
use lib3h_protocol::DidWork;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, RwLock},
};
use url::Url;

//--------------------------------------------------------------------------------------------------
// Memory Server MAP
//--------------------------------------------------------------------------------------------------

/// Type for holding a map of 'url -> InMemoryServer'
type MemoryServerMap = HashMap<Url, Mutex<MemoryServer>>;

// this is the actual memory space for our in-memory servers
lazy_static! {
    pub(crate) static ref MEMORY_SERVER_MAP: RwLock<MemoryServerMap> = RwLock::new(HashMap::new());
}

/// Add new MemoryServer to the global server map
pub fn set_server(uri: &Url) -> TransportResult<()> {
    debug!("MemoryServer::set_server: {}", uri);
    // Create server with that name if it doesn't already exist
    let mut server_map = MEMORY_SERVER_MAP.write().unwrap();
    if server_map.contains_key(uri) {
        return Err(TransportError::new("Server already exist".to_string()));
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
pub struct MemoryServer {
    /// Address of this server
    this_uri: Url,
    /// Inboxes for payloads from each of its connections.
    inbox_map: HashMap<Url, VecDeque<Vec<u8>>>,
    /// Inbox of connection state change requests
    /// (true = incoming connection, false = connection closed)
    connection_inbox: Vec<(Url, bool)>,
    /// Store of all established connections
    inbound_connections: HashMap<Url, ConnectionId>,
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
            inbound_connections: HashMap::new(),
        }
    }

    pub fn get_inbound_uri(&self, arg_id: &ConnectionIdRef) -> Option<&Url> {
        self.inbound_connections
            .iter()
            .find(|(_, id)| *id == arg_id)
            .map(|(uri, _)| uri)
    }

    /// Another node requested to connect with us.
    /// This creates a new connection: An inbox is created for receiving payloads from this requester.
    /// This also generates a request for us to connect to the other node in the other way.
    pub fn connect(&mut self, requester_uri: &Url, connection_id: &ConnectionIdRef) -> TransportResult<()> {
        info!("(MemoryServer) {} creates inbox for {} ({})", self.this_uri, requester_uri, connection_id);
        if requester_uri == &self.this_uri {
            return Err(TransportError::new(format!(
                "Server {} cannot connect to self",
                self.this_uri,
            )));
        }
        if self.inbox_map.contains_key(requester_uri) {
            return Err(TransportError::new(format!(
                "Server {}, is already connected to {}",
                self.this_uri, requester_uri,
            )));
        }
        let _ = self
            .inbox_map
            .insert(requester_uri.clone(), VecDeque::new());
        // Notify our TransportMemory (so it can connect back)
        self.connection_inbox
            .push((requester_uri.clone(), true));
        self.inbound_connections.insert(requester_uri.clone(), connection_id.to_string());
        Ok(())
    }

    /// Close a connection
    pub fn close(&mut self, other_uri: &Url) -> TransportResult<()> {
        info!("(MemoryServer {}).close({})", self.this_uri, other_uri);
        // delete this connectionId's inbox
        let res = self.inbox_map.remove(other_uri);
        if res.is_none() {
            return Err(TransportError::new(format!(
                "connectionId '{}' unknown for server {}",
                other_uri, self.this_uri
            )));
        }
        trace!("(MemoryServer {}). close event", self.this_uri);
        // Notify our TransportMemory
        self.connection_inbox.push((other_uri.clone(), false));
        // Locally remove connection
        // self.inbound_connections.remove(other_uri);
        Ok(())
    }

    /// Receive payload from another node, i.e. fill our inbox for this connectionId
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
    pub fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        trace!("(MemoryServer {}).process()", self.this_uri);
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Process connection inbox
        for (uri, is_new) in self.connection_inbox.iter() {
            trace!("(MemoryServer {}). connection_inbox: {} | {}", self.this_uri, uri, is_new);
            let id = self.inbound_connections.get(uri)
                .expect("Should always have id for a connected uri (connection)").to_string();
            let event = if *is_new {
                TransportEvent::IncomingConnectionEstablished(id.to_string())
            } else {
                self.inbound_connections.remove(uri);
                TransportEvent::ConnectionClosed(id.to_string())
            };
            trace!("(MemoryServer {}). connection: {:?}", self.this_uri, event);
            outbox.push(event);
            did_work = true;
        }
        self.connection_inbox.clear();
        // Process msg inboxes
        for (uri, inbox) in self.inbox_map.iter_mut() {
            let id = self.inbound_connections.get(uri)
                .expect("Should always have id for a connected uri (msg)");
            loop {
                let payload = match inbox.pop_front() {
                    None => break,
                    Some(msg) => msg,
                };
                did_work = true;
                trace!("(MemoryServer {}) received: {:?}", self.this_uri, payload);
                let evt = TransportEvent::ReceivedData(id.to_string(), payload);
                outbox.push(evt);
            }
        }
        Ok((did_work, outbox))
    }
}
