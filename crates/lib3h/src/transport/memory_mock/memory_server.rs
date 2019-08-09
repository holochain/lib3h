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

pub type ServerInst = std::sync::Arc<Mutex<MemoryServer>>;

/// Type for holding a map of 'url -> InMemoryServer'
type MemoryServerMap = HashMap<Url, std::sync::Weak<Mutex<MemoryServer>>>;

// this is the actual memory space for our in-memory servers
lazy_static! {
    pub(crate) static ref MEMORY_SERVER_MAP: RwLock<MemoryServerMap> = RwLock::new(HashMap::new());
}

/// Add new MemoryServer to the global server map
pub fn ensure_server(uri: &Url) -> TransportResult<ServerInst> {
    debug!("MemoryServer::set_server: {}", uri);
    let mut server_map = MEMORY_SERVER_MAP.write().unwrap();

    // make sure we keep a STRONG reference around for the first one,
    // or it'll get cleaned up before we even send it out.
    let mut out = None;

    let tmp = server_map.entry(uri.clone()).or_insert_with(|| {
        let s = std::sync::Arc::new(Mutex::new(MemoryServer::new(uri)));
        let r = std::sync::Arc::downgrade(&s);
        out = Some(s);
        r
    });

    Ok(match out {
        Some(s) => s,
        None => std::sync::Weak::upgrade(tmp).unwrap(),
    })
}

pub struct ServerRef(std::sync::Arc<Mutex<MemoryServer>>);
impl ServerRef {
    pub fn get(&self) -> std::sync::MutexGuard<'_, MemoryServer> {
        self.0.lock().expect("can read MemoryServer")
    }
}

pub fn read_ref(uri: &Url) -> TransportResult<ServerRef> {
    let server_map = MEMORY_SERVER_MAP.read().expect("map exists");
    let maybe_server = server_map.get(uri);
    if maybe_server.is_none() {
        return Err(TransportError::new(format!("No Memory server at {}", uri)));
    }
    Ok(ServerRef(
        std::sync::Weak::upgrade(maybe_server.unwrap()).expect("server still exists"),
    ))
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
    connection_inbox: Vec<(ConnectionId, bool)>,
    /// Store of all established connections
    inbound_connections: HashMap<Url, ConnectionId>,
}

impl Drop for MemoryServer {
    fn drop(&mut self) {
        trace!("(MemoryServer) dropped: {:?}", self.this_uri);
        let mut server_map = MEMORY_SERVER_MAP.write().unwrap();
        if !server_map.contains_key(&self.this_uri) {
            panic!("Server doesn't exist");
        }
        server_map.remove(&self.this_uri);
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
    pub fn request_connect(
        &mut self,
        other_uri: &Url,
        in_cid: &ConnectionIdRef,
    ) -> TransportResult<()> {
        info!(
            "(MemoryServer) {} creates inbox for {} ({})",
            self.this_uri, other_uri, in_cid
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
        // Establish inbound connection
        let prev = self.inbox_map.insert(other_uri.clone(), VecDeque::new());
        assert!(prev.is_none());
        self.inbound_connections
            .insert(other_uri.clone(), in_cid.to_string());
        // Notify our TransportMemory (so it can connect back)
        self.connection_inbox.push((in_cid.to_string(), true));
        // Done
        Ok(())
    }

    /// Another node closes its connection with us
    pub fn request_close(&mut self, other_uri: &Url) -> TransportResult<()> {
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
        // Remove inbound connection
        let in_cid = self
            .inbound_connections
            .remove(other_uri)
            .expect("Should have connectionId for this uri");
        // Notify our TransportMemory
        self.connection_inbox.push((in_cid.clone(), false));
        // Done
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
        for (in_cid, is_new) in self.connection_inbox.iter() {
            trace!(
                "(MemoryServer {}). connection_inbox: {} | {}",
                self.this_uri,
                in_cid,
                is_new,
            );
            let event = if *is_new {
                TransportEvent::IncomingConnectionEstablished(in_cid.to_string())
            } else {
                TransportEvent::ConnectionClosed(in_cid.to_string())
            };
            trace!("(MemoryServer {}). connection: {:?}", self.this_uri, event);
            outbox.push(event);
            did_work = true;
        }
        self.connection_inbox.clear();
        // Process msg inboxes
        for (uri, inbox) in self.inbox_map.iter_mut() {
            let id = self
                .inbound_connections
                .get(uri)
                .expect("Should always have id for a connected uri (msg)");
            loop {
                let payload = match inbox.pop_front() {
                    None => break,
                    Some(msg) => msg,
                };
                did_work = true;
                trace!("(MemoryServer {}) received: {:?}", self.this_uri, payload);
                let evt = TransportEvent::ReceivedData(id.to_string(), payload.into());
                outbox.push(evt);
            }
        }
        Ok((did_work, outbox))
    }
}
