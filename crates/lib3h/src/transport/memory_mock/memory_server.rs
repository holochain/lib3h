use crate::transport::{
    error::{TransportError, TransportResult},
    protocol::TransportEvent,
    ConnectionId, ConnectionIdRef,
};
use lib3h_protocol::DidWork;
use std::{
    collections::{HashMap, HashSet, VecDeque},
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
pub fn unset_server(url: &Url) -> TransportResult<()> {
    // Create server with that name if it doesn't already exist
    let mut server_map = MEMORY_SERVER_MAP.write().unwrap();
    if !server_map.contains_key(url) {
        return Err(TransportError::new("Server doesn't exist".to_string()));
    }
    server_map.remove(url);
    Ok(())
}

//--------------------------------------------------------------------------------------------------
// Memory Server
//--------------------------------------------------------------------------------------------------

pub struct MemoryServer {
    /// Address of this server
    uri: Url,
    /// Inboxes for payloads from each of its connections.
    inbox_map: HashMap<ConnectionId, VecDeque<Vec<u8>>>,
    /// Inbox of connection state change requests
    /// (true = incoming connection, false = connection closed)
    connection_inbox: Vec<(ConnectionId, bool)>,
    /// Store of all established connections
    connections: HashSet<ConnectionId>,
}

impl MemoryServer {
    /// Constructor
    pub fn new(uri: &Url) -> Self {
        MemoryServer {
            uri: uri.clone(),
            inbox_map: HashMap::new(),
            connection_inbox: Vec::new(),
            connections: HashSet::new(),
        }
    }

    pub fn has_connection(&self, id: &ConnectionIdRef) -> bool {
        self.connections.contains(id)
    }

    /// Another node requested to connect with us.
    /// This creates a new connection: An inbox is created for receiving payloads from this requester.
    /// This also generates a request for us to connect to the other node in the other way.
    pub fn connect(&mut self, requester_uri: &ConnectionIdRef) -> TransportResult<()> {
        info!(
            "(MemoryServer) {} creates inbox for {}",
            self.uri, requester_uri
        );
        if self.inbox_map.contains_key(requester_uri) {
            return Err(TransportError::new(format!(
                "Server {}, is already connected to {}",
                self.uri, requester_uri,
            )));
        }
        let res = self
            .inbox_map
            .insert(requester_uri.to_string(), VecDeque::new());
        if res.is_some() {
            return Err(TransportError::new("connectionId already used".to_string()));
        }
        // Notify our TransportMemory (so it can connect back)
        self.connection_inbox.push((requester_uri.to_string(), true));
        self.connections.insert(requester_uri.to_string());
        Ok(())
    }

    /// Close a connection
    pub fn close(&mut self, id: &ConnectionIdRef) -> TransportResult<()> {
        info!("(MemoryServer {}).close({})", self.uri, id);
        // delete this connectionId's inbox
        let res = self.inbox_map.remove(id);
        if res.is_none() {
            return Err(TransportError::new(format!(
                "connectionId '{}' unknown for server {}",
                id, self.uri
            )));
        }
        // Notify our TransportMemory
        self.connection_inbox.push((id.to_string(), false));
        // Locally remove connection
        self.connections.remove(&id.to_string());
        Ok(())
    }

    /// Receive payload from another node, i.e. fill our inbox for this connectionId
    pub fn post(&mut self, from_id: &ConnectionIdRef, payload: &[u8]) -> TransportResult<()> {
        let maybe_inbox = self.inbox_map.get_mut(from_id);
        if let None = maybe_inbox {
            return Err(TransportError::new(format!(
                "(MemoryServer {}) Unknown connectionId {}",
                self.uri, from_id
            )));
        }
        maybe_inbox.unwrap().push_back(payload.to_vec());
        Ok(())
    }

    /// Process all inboxes: payload inboxes and incoming connections inbox.
    /// Return a TransportEvent::ReceivedData for each payload processed and
    /// a TransportEvent::IncomingConnectionEstablished for each incoming connection.
    pub fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        trace!("(MemoryServer {}).process()", self.uri);
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Process connection inbox
        for (uri, is_new) in self.connection_inbox.iter() {
            let event = if *is_new {
                TransportEvent::IncomingConnectionEstablished(uri.to_string())
            } else {
                TransportEvent::ConnectionClosed(uri.to_string())
            };
            outbox.push(event);
            did_work = true;
        }
        self.connection_inbox.clear();
        // Process msg inboxes
        for (id, inbox) in self.inbox_map.iter_mut() {
            loop {
                let payload = match inbox.pop_front() {
                    None => break,
                    Some(msg) => msg,
                };
                did_work = true;
                trace!("(MemoryServer {}) received: {:?}", self.uri, payload);
                let evt = TransportEvent::ReceivedData(id.clone(), payload);
                outbox.push(evt);
            }
        }
        Ok((did_work, outbox))
    }
}
