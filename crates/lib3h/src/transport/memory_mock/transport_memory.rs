use crate::transport::{
    error::{TransportError, TransportResult},
    memory_mock::memory_server,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    ConnectionId, ConnectionIdRef,
};
use lib3h_protocol::DidWork;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
};
use url::Url;
/// Transport for mocking network layer in-memory
/// Binding creates a MemoryServer at url that can be accessed by other nodes
pub struct TransportMemory {
    /// Reference to our memory servers
    memory_servers: Vec<Arc<Mutex<memory_server::MemoryServer>>>,
    /// Commands sent to us by owner for async processing
    cmd_inbox: VecDeque<TransportCommand>,
    /// Addresses (url-ish) of all our servers
    my_servers: HashSet<Url>,
    /// Mapping of connectionId -> serverUrl
    outbound_connection_map: HashMap<ConnectionId, Url>,
    /// Mapping of in:connectionId -> out:connectionId
    inbound_connection_map: HashMap<ConnectionId, ConnectionId>,
    /// Counter for generating new connectionIds
    n_id: u32,
    own_id: u32,
    /// My peer uri on the network layer
    maybe_my_uri: Option<Url>,
}

lazy_static! {
    /// Counter of the number of TransportMemory that spawned
    static ref TRANSPORT_COUNT: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
}

impl TransportMemory {
    pub fn new() -> Self {
        let mut tc = TRANSPORT_COUNT
            .lock()
            .expect("could not lock transport count mutex");
        *tc += 1;
        TransportMemory {
            memory_servers: vec![],
            cmd_inbox: VecDeque::new(),
            my_servers: HashSet::new(),
            outbound_connection_map: HashMap::new(),
            inbound_connection_map: HashMap::new(),
            n_id: 0,
            own_id: *tc,
            maybe_my_uri: None,
        }
    }

    pub fn name(&self) -> &str {
        match &self.maybe_my_uri {
            None => "",
            Some(uri) => uri.as_str(),
        }
    }

    pub fn is_bound(&self, id: &ConnectionIdRef) -> bool {
        match &self.maybe_my_uri {
            None => false,
            Some(uri) => {
                let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
                server_map
                    .get(uri)
                    .map(|server| {
                        std::sync::Weak::upgrade(server)
                            .expect("server still exists")
                            .lock()
                            .unwrap()
                            .get_inbound_uri(id)
                            .is_some()
                    })
                    .unwrap_or(false)
            }
        }
    }
}

impl Drop for TransportMemory {
    fn drop(&mut self) {
        // Close all connections
        self.close_all().ok();
    }
}
/// Compose Transport
impl Transport for TransportMemory {
    /// Get list of known connectionIds
    fn connection_id_list(&self) -> TransportResult<Vec<ConnectionId>> {
        Ok(self
            .outbound_connection_map
            .keys()
            .map(|id| id.to_string())
            .collect())
    }

    /// get uri from a connectionId
    fn get_uri(&self, id: &ConnectionIdRef) -> Option<Url> {
        let res = self.outbound_connection_map.get(&id.to_string());
        res.map(|url| url.clone()).or_else(|| {
            if self.is_bound(id) {
                match &self.maybe_my_uri {
                    Some(uri) => Some(uri.clone()),
                    None => None,
                }
            } else {
                None
            }
        })
    }

    /// Connect to another node's "bind".
    /// Get server from the uri and connect to it with a new connectionId for ourself.
    fn connect(&mut self, uri: &Url) -> TransportResult<ConnectionId> {
        // Check if already connected
        let maybe_cid = self
            .outbound_connection_map
            .iter()
            .find(|(_, cur_uri)| *cur_uri == uri);
        if let Some((cid, _)) = maybe_cid {
            return Ok(cid.clone());
        }
        // Get my uri
        let my_uri = match &self.maybe_my_uri {
            None => {
                return Err(TransportError::new(
                    "Must bind before connecting".to_string(),
                ));
            }
            Some(u) => u,
        };
        // Get other node's server
        let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
        let maybe_server = server_map.get(uri);
        if let None = maybe_server {
            return Err(TransportError::new(format!(
                "No Memory server at this url address: {}",
                uri
            )));
        }
        // Generate and store a connectionId to act like other Transport types
        self.n_id += 1;
        let id = format!("mem_conn_{}_{}", self.own_id, self.n_id);
        self.outbound_connection_map.insert(id.clone(), uri.clone());
        // Connect to it
        let server = std::sync::Weak::upgrade(maybe_server.unwrap()).expect("server still exists");
        let mut server = server.lock().unwrap();
        server.request_connect(my_uri, &id)?;
        Ok(id)
    }

    /// Notify other server on that connectionId that we are closing connection and
    /// locally clear that connectionId.
    fn close(&mut self, id: &ConnectionIdRef) -> TransportResult<()> {
        trace!("TransportMemory[{}].close({})", self.own_id, id);
        if self.maybe_my_uri.is_none() {
            return Err(TransportError::new(
                "Cannot close a connection before bounding".to_string(),
            ));
        }
        let my_uri = self.maybe_my_uri.clone().unwrap();
        // Get the other node's uri on that connection
        let maybe_other_uri = self.outbound_connection_map.get(id);
        if let None = maybe_other_uri {
            return Err(TransportError::new(format!("Unknown connectionId: {}", id)));
        }
        let other_uri = maybe_other_uri.unwrap();
        // Get other node's server
        let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
        let maybe_other_server = server_map.get(other_uri);
        if let None = maybe_other_server {
            return Err(TransportError::new(format!(
                "No Memory server at this url: {}",
                other_uri,
            )));
        }
        let other_server =
            std::sync::Weak::upgrade(maybe_other_server.unwrap()).expect("server still exists");
        let mut other_server = other_server.lock().unwrap();
        // Tell it we closed connection with it
        let _ = other_server.request_close(&my_uri);
        // Locally remove connection
        self.outbound_connection_map.remove(id);
        // Done
        Ok(())
    }

    /// Close all known connectionIds
    fn close_all(&mut self) -> TransportResult<()> {
        let id_list = self.connection_id_list()?;
        for id in id_list {
            let res = self.close(&id);
            if let Err(e) = res {
                warn!("Closing connection {} failed: {:?}", id, e);
            }
        }
        Ok(())
    }

    /// Send payload to known connectionIds in `id_list`
    fn send(&mut self, id_list: &[&ConnectionIdRef], payload: &[u8]) -> TransportResult<()> {
        if self.maybe_my_uri.is_none() {
            return Err(TransportError::new(
                "Cannot send before bounding".to_string(),
            ));
        }
        let my_uri = self.maybe_my_uri.clone().unwrap();
        for id in id_list {
            // Get the other node's uri on that connection
            let maybe_uri = self.outbound_connection_map.get(*id);
            if let None = maybe_uri {
                warn!("No known connection for connectionId: {}", id);
                continue;
            }
            let uri = maybe_uri.unwrap();
            // Get the other node's server
            let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
            let maybe_server = server_map.get(uri);
            if let None = maybe_server {
                return Err(TransportError::new(format!(
                    "No Memory server at this url address: {}",
                    uri
                )));
            }
            trace!("(TransportMemory).send() {} | {}", uri, payload.len());
            let server =
                std::sync::Weak::upgrade(maybe_server.unwrap()).expect("server still exists");
            let mut server = server.lock().unwrap();
            // Send it data from us
            server
                .post(&my_uri, payload)
                .expect("Post on memory server should work");
        }
        Ok(())
    }

    /// Send to all known connectionIds
    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        let id_list = self.connection_id_list()?;
        for id in id_list {
            self.send(&[id.as_str()], payload)?;
        }
        Ok(())
    }

    /// Add Command to inbox
    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.cmd_inbox.push_back(command);
        Ok(())
    }

    /// Create a new server inbox for myself
    fn bind(&mut self, uri: &Url) -> TransportResult<Url> {
        let bounded_uri = Url::parse(format!("{}_bound", uri).as_str()).unwrap();
        self.maybe_my_uri = Some(bounded_uri.clone());
        self.memory_servers
            .push(memory_server::ensure_server(&bounded_uri)?);
        self.my_servers.insert(bounded_uri.clone());
        Ok(bounded_uri.clone())
    }

    /// Process my TransportCommand inbox and all my server inboxes
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        // trace!("(TransportMemory).process()");
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Process TransportCommand inbox
        loop {
            let cmd = match self.cmd_inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            let res = self.serve_TransportCommand(&cmd);
            if let Ok(mut output) = res {
                did_work = true;
                outbox.append(&mut output);
            }
        }
        // Process my Servers: process IncomingConnectionEstablished first
        let mut to_connect_list: Vec<(Url, ConnectionId)> = Vec::new();
        let mut output = Vec::new();
        for my_server_uri in &self.my_servers {
            let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
            let my_server = std::sync::Weak::upgrade(
                server_map
                    .get(my_server_uri)
                    .expect("My server should exist."),
            )
            .expect("server still exists");
            let mut my_server = my_server.lock().unwrap();
            let (success, event_list) = my_server.process()?;
            if success {
                did_work = true;

                for event in event_list {
                    if let TransportEvent::IncomingConnectionEstablished(in_cid) = event {
                        let to_connect_uri = my_server
                            .get_inbound_uri(&in_cid)
                            .expect("Should always have uri");
                        to_connect_list.push((to_connect_uri.clone(), in_cid.clone()));
                    } else {
                        output.push(event);
                    }
                }
            }
        }
        // Connect back to received connections if not already connected to them
        for (uri, in_cid) in to_connect_list {
            trace!("(TransportMemory) {} <- {:?}", uri, self.maybe_my_uri);
            let out_cid = self.connect(&uri)?;
            self.inbound_connection_map
                .insert(in_cid.clone(), out_cid.clone());
            // Note: Add IncomingConnectionEstablished events at start of outbox
            // so they can be processed first.
            outbox.insert(0, TransportEvent::IncomingConnectionEstablished(out_cid));
        }
        // process other messages
        for event in output {
            match event {
                TransportEvent::ConnectionClosed(in_cid) => {
                    // convert inbound connectionId to outbound connectionId.
                    // let out_cid = self.inbound_connection_map.get(&in_cid).expect("Should have outbound at this stage");
                    let out_cid = self
                        .inbound_connection_map
                        .remove(&in_cid)
                        .expect("Should have outbound at this stage");
                    // close will fail as other side isn't there anymore
                    let _ = self.close(&out_cid);
                    outbox.push(TransportEvent::ConnectionClosed(out_cid.to_string()));
                }
                TransportEvent::ReceivedData(in_cid, data) => {
                    // convert inbound connectionId to outbound connectionId.
                    let out_cid = self
                        .inbound_connection_map
                        .get(&in_cid)
                        .expect("Should have outbound at this stage");
                    outbox.push(TransportEvent::ReceivedData(
                        out_cid.to_string(),
                        data.clone(),
                    ));
                }
                // We are not expecting anything else from the MemoryServer
                _ => unreachable!(),
            }
        }
        // Done
        Ok((did_work, outbox))
    }
}

impl TransportMemory {
    /// Process a TransportCommand: Call the corresponding method and possibily return some Events.
    /// Return a list of TransportEvents to owner.
    #[allow(non_snake_case)]
    fn serve_TransportCommand(
        &mut self,
        cmd: &TransportCommand,
    ) -> TransportResult<Vec<TransportEvent>> {
        debug!(">>> '(TransportMemory)' recv cmd: {:?}", cmd);
        // Note: use same order as the enum
        match cmd {
            TransportCommand::Connect(url) => {
                let id = self.connect(url)?;
                let evt = TransportEvent::ConnectResult(id);
                Ok(vec![evt])
            }
            TransportCommand::Send(id_list, payload) => {
                let mut id_ref_list = Vec::with_capacity(id_list.len());
                for id in id_list {
                    id_ref_list.push(id.as_str());
                }
                let _id = self.send(&id_ref_list, payload)?;
                Ok(vec![])
            }
            TransportCommand::SendAll(payload) => {
                let _id = self.send_all(payload)?;
                Ok(vec![])
            }
            TransportCommand::Close(id) => {
                self.close(id)?;
                let evt = TransportEvent::ConnectionClosed(id.to_string());
                Ok(vec![evt])
            }
            TransportCommand::CloseAll => {
                self.close_all()?;
                let mut outbox = Vec::new();
                for (id, _url) in &self.outbound_connection_map {
                    let evt = TransportEvent::ConnectionClosed(id.to_string());
                    outbox.push(evt);
                }
                Ok(outbox)
            }
            TransportCommand::Bind(url) => {
                self.bind(url)?;
                Ok(vec![])
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn can_rebind() {
        let mut transport = TransportMemory::new();
        let bind_url = url::Url::parse("mem://can_rebind").unwrap();
        assert!(transport.bind(&bind_url).is_ok());
        assert!(transport.bind(&bind_url).is_ok());
    }

}
