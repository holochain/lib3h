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
    /// Inbox of new inbound connections
    new_conn_inbox: Vec<ConnectionId>,
    connection_ids: HashSet<ConnectionId>,
}

impl MemoryServer {
    /// Constructor
    pub fn new(uri: &Url) -> Self {
        MemoryServer {
            uri: uri.clone(),
            inbox_map: HashMap::new(),
            new_conn_inbox: Vec::new(),
            connection_ids: HashSet::new(),
        }
    }

    pub fn has_connection(&self, id: &ConnectionIdRef) -> bool {
        self.connection_ids.contains(id)
    }

    /// Create an inbox for this new sender
    /// Will connect the other way.
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
        // Notify our TransportMemory to connect back
        self.new_conn_inbox.push(requester_uri.to_string());
        self.connection_ids.insert(requester_uri.to_string());
        Ok(())
    }

    /// Delete this connectionId's inbox
    pub fn close(&mut self, id: &ConnectionIdRef) -> TransportResult<()> {
        info!("(MemoryServer {}).close({})", self.uri, id);
        let res = self.inbox_map.remove(id);
        if res.is_none() {
            return Err(TransportError::new(format!(
                "connectionId '{}' unknown for server {}",
                id, self.uri
            )));
        }
        self.connection_ids.remove(&id.to_string());
        // TODO #159 - Maybe process here whatever is left in the inbox?
        Ok(())
    }

    /// Add payload to connectionId's inbox
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

    /// Process all inboxes.
    /// Return a TransportEvent::Received for each payload processed.
    pub fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        trace!("(MemoryServer {}).process()", self.uri);
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Process connexion inbox
        for uri in self.new_conn_inbox.iter() {
            outbox.push(TransportEvent::ConnectResult(uri.to_string()));
            did_work = true;
        }
        self.new_conn_inbox.clear();
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
