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

pub fn set_server(url: &str) -> TransportResult<()> {
    // Create server with that name if it doesn't already exist
    let mut server_map = MEMORY_SERVER_MAP.write().unwrap();
    if server_map.contains_key(url) {
        return Err(TransportError::new("Server already exist".to_string()));
    }
    server_map.insert(url.to_string(), Mutex::new(MemoryServer::new(url)));
    Ok(())
}

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
    uri: String,
    inbox_map: HashMap<TransportId, VecDeque<Vec<u8>>>,
}

impl MemoryServer {
    pub fn new(uri: &str) -> Self {
        MemoryServer {
            uri: uri.to_string(),
            inbox_map: HashMap::new(),
        }
    }

    pub fn connect(&mut self, id: &TransportIdRef) -> TransportResult<()> {
        if self.inbox_map.contains_key(id) {
            return Err(TransportError::new(format!(
                "TransportId '{}' already used for server {}",
                id, self.uri
            )));
        }
        self.inbox_map
            .insert(id.to_string(), VecDeque::new())
            .expect("TransportId should be unique");
        Ok(())
    }

    pub fn close(&mut self, _id: &TransportIdRef) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    pub fn post(&mut self, id: &TransportIdRef, payload: &[u8]) -> TransportResult<()> {
        let maybe_inbox = self.inbox_map.get_mut(id);
        if let None = maybe_inbox {
            return Err(TransportError::new(format!("Unknown TransportId {}", id)));
        }
        maybe_inbox.unwrap().push_back(payload.to_vec());
        Ok(())
    }

    // FIXME
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
