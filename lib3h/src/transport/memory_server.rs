use crate::transport::{
    error::TransportResult,
    memory_server::MemoryServer,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    TransportId, TransportIdRef,
};
use lib3h_protocol::{DidWork, Lib3hResult};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet, VecDeque},
    convert::TryFrom,
    sync::{mpsc, Mutex, MutexGuard, RwLock},
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

pub fn get_server(url: &string) -> Option<MutexGuard<MemoryServer>> {
    let server_map = MEMORY_SERVER_MAP.read().unwrap();
    let maybe_server = server_map.get(url);

    match maybe_server {
        None => None,
        Some(server) => server.lock().unwrap(),
    }
}

pub fn set_server(url: &string) -> Lib3hResult<()> {
    // Create server with that name if it doesn't already exist
    let mut server_map = MEMORY_SERVER_MAP.write().unwrap();
    if server_map.contains_key(&url) {
        return Err(format_err!("Server already exist"));
    }
    server_map.insert(url, Mutex::new(MemoryServer::new(url)));
    Ok(())
}

pub fn unset_server(url: &string) -> Lib3hResult<()> {
    // Create server with that name if it doesn't already exist
    let mut server_map = MEMORY_SERVER_MAP.write().unwrap();
    if !server_map.contains_key(&url) {
        return Err(format_err!("Server doesn't exist"));
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
    n_id: u32,
}

impl MemoryServer {
    pub fn new(uri: &str) -> Self {
        MemoryServer {
            uri: uri.to_string(),
            inbox_map: HashMap::new(),
            n_id: 0,
        }
    }

    fn connect(&mut self, id: TransportId) -> TransportResult<()> {
        if self.inbox_map.contains(id) {
            return Err(format_err!(
                "TransportId '{}' already used for server {}",
                id,
                self.uri
            ));
        }
        self.inbox_map.insert(id, VecDeque::new())?;
        Ok(())
    }

    fn close(&mut self, _id: TransportId) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    fn post(&mut self, id: TransportId, payload: &[u8]) -> TransportResult<()> {
        let inbox = self.inbox_map.get(id)?;
        inbox.push_back(payload);
        Ok(())
    }

    // FIXME
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Process inboxes
        for (id, inbox) in self.inbox_map {
            loop {
                let payload = match self.inbox.pop_front() {
                    None => break,
                    Some(msg) => msg,
                };
                did_work = true;
                let evt = TransportEvent::Received(id, payload);
                outbox.push(evt);
            }
        }
        Ok((did_work, outbox))
    }
}
