use crate::transport::{
    error::{TransportError, TransportResult},
    protocol::TransportEvent,
    TransportId, TransportIdRef,
};
use lib3h_protocol::DidWork;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, RwLock},
};

//--------------------------------------------------------------------------------------------------
// Memory Server MAP
//--------------------------------------------------------------------------------------------------

/// Type for holding a map of 'url -> InMemoryServer'
type MemoryServerMap = HashMap<String, Mutex<MemoryServer>>;

/// this is the actual memory space for our in-memory servers
lazy_static! {
    pub(crate) static ref MEMORY_SERVER_MAP: RwLock<MemoryServerMap> = RwLock::new(HashMap::new());
}

/// Add new MemoryServer to the global server map
pub fn set_server(url: &str) -> TransportResult<()> {
    // println!("(log.d) set_server: {}", url);
    // Create server with that name if it doesn't already exist
    let mut server_map = MEMORY_SERVER_MAP.write().unwrap();
    if server_map.contains_key(url) {
        return Err(TransportError::new("Server already exist".to_string()));
    }
    server_map.insert(url.to_string(), Mutex::new(MemoryServer::new(url)));
    Ok(())
}

/// Remove a MemoryServer from the global server map
pub fn unset_server(url: &str) -> TransportResult<()> {
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
    uri: String,
    /// Inboxes for payloads from each of its connections.
    inbox_map: HashMap<TransportId, VecDeque<Vec<u8>>>,
}

impl MemoryServer {
    /// Constructor
    pub fn new(uri: &str) -> Self {
        MemoryServer {
            uri: uri.to_string(),
            inbox_map: HashMap::new(),
        }
    }

    /// Create new inbox for this new transportId
    pub fn connect(&mut self, id: &TransportIdRef) -> TransportResult<()> {
        println!("[i] {}.connect({})", self.uri, id);
        if self.inbox_map.contains_key(id) {
            return Err(TransportError::new(format!(
                "TransportId '{}' already used for server {}",
                id, self.uri
            )));
        }
        let res = self.inbox_map.insert(id.to_string(), VecDeque::new());
        if res.is_some() {
            return Err(TransportError::new("TransportId already used".to_string()));
        }
        Ok(())
    }

    /// Delete this transportId's inbox
    pub fn close(&mut self, id: &TransportIdRef) -> TransportResult<()> {
        println!("[i] {}.close({})", self.uri, id);
        let res = self.inbox_map.remove(id);
        if res.is_none() {
            return Err(TransportError::new(format!(
                "TransportId '{}' unknown for server {}",
                id, self.uri
            )));
        }
        // TODO: Should we process here whatever is left in the inbox?
        Ok(())
    }

    /// Add payload to transportId's inbox
    pub fn post(&mut self, id: &TransportIdRef, payload: &[u8]) -> TransportResult<()> {
        let maybe_inbox = self.inbox_map.get_mut(id);
        if let None = maybe_inbox {
            return Err(TransportError::new(format!("Unknown TransportId {}", id)));
        }
        maybe_inbox.unwrap().push_back(payload.to_vec());
        Ok(())
    }

    /// Process all inboxes.
    /// Return a TransportEvent::Received for each payload processed.
    pub fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Process inboxes
        for (id, inbox) in self.inbox_map.iter_mut() {
            loop {
                let payload = match inbox.pop_front() {
                    None => break,
                    Some(msg) => msg,
                };
                did_work = true;
                let evt = TransportEvent::Received(id.clone(), payload);
                outbox.push(evt);
            }
        }
        Ok((did_work, outbox))
    }
}
