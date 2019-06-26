use crate::transport::{
    error::{TransportError, TransportResult},
    memory_mock::memory_server,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    TransportId, TransportIdRef,
};
use lib3h_protocol::DidWork;
use std::collections::{HashMap, HashSet, VecDeque};
use url::Url;

/// Transport for mocking network layer in-memory
/// Binding creates a MemoryServer at url that can be accessed by other nodes
pub struct TransportMemory {
    /// Commands sent to us by owner for async processing
    cmd_inbox: VecDeque<TransportCommand>,
    /// Addresses (url-ish) of all our servers
    my_servers: HashSet<Url>,
    /// Mapping of transportId -> serverUrl
    connections: HashMap<TransportId, Url>,
    /// Counter for generating new transportIds
    n_id: u32,
    ///// My peer address on the network layer
    /// My peer transport
    maybe_my_uri: Option<Url>,
}

impl TransportMemory {
    pub fn new() -> Self {
        TransportMemory {
            cmd_inbox: VecDeque::new(),
            my_servers: HashSet::new(),
            connections: HashMap::new(),
            n_id: 0,
            maybe_my_uri: None,
        }
    }

    pub fn name(&self) -> &str {
        match &self.maybe_my_uri {
            None => "",
            Some(uri) => uri.as_str(),
        }
    }
}

/// Compose Transport
impl Transport for TransportMemory {
    /// Get list of known transportIds
    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        Ok(self.connections.keys().map(|id| id.to_string()).collect())
    }

    /// get uri from a transportId
    fn get_uri(&self, id: &TransportIdRef) -> Option<Url> {
        let res = self.connections.get(&id.to_string());
        res.map(|url| url.clone())
    }

    /// Connect to another node's "bind".
    /// Get server from the uri and connect to it with a new transportId for ourself.
    fn connect(&mut self, uri: &Url) -> TransportResult<TransportId> {
        // Self must have uri
        let my_uri = match &self.maybe_my_uri {
            None => {
                return Err(TransportError::new(
                    "Must bind before connecting".to_string(),
                ));
            }
            Some(u) => u,
        };

        // println!("[d] ---- connect: {} -> {}", my_uri, uri);
        let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
        let maybe_server = server_map.get(uri);
        if let None = maybe_server {
            return Err(TransportError::new(format!(
                "No Memory server at this url address: {}",
                uri
            )));
        }
        // Generate ConnectionId
        self.n_id += 1;
        let id = format!("mem_conn_{}", self.n_id);
        // Get other's server and connect us to it by using our new ConnectionId.
        let mut server = maybe_server.unwrap().lock().unwrap();
        server.connect(&my_uri.as_str())?;

        // Store ConnectionId
        self.connections.insert(id.clone(), uri.clone());
        Ok(id)
    }

    /// Notify server on that transportId that we are closing connection and clear that transportId.
    fn close(&mut self, id: &TransportIdRef) -> TransportResult<()> {
        let maybe_url = self.connections.get(id);
        if let None = maybe_url {
            return Err(TransportError::new(format!(
                "No known connection for TransportId {}",
                id
            )));
        }
        let url = maybe_url.unwrap();
        // Get other node's server
        let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
        let maybe_server = server_map.get(url);
        if let None = maybe_server {
            return Err(TransportError::new(format!(
                "No Memory server at this url: {}",
                url,
            )));
        }
        let mut server = maybe_server.unwrap().lock().unwrap();
        // Tell it to close connection
        server.close(&id)?;
        // Locally remove connection
        self.connections.remove(id);
        Ok(())
    }

    /// Close all known transportIds
    fn close_all(&mut self) -> TransportResult<()> {
        let id_list = self.transport_id_list()?;
        for id in id_list {
            self.close(&id)?;
        }
        Ok(())
    }

    /// Send payload to known transportIds in `id_list`
    fn send(&mut self, id_list: &[&TransportIdRef], payload: &[u8]) -> TransportResult<()> {
        for id in id_list {
            let maybe_uri = self.connections.get(*id);
            if let None = maybe_uri {
                println!("[w] No known connection for TransportId: {}", id);
                continue;
            }
            let uri = maybe_uri.unwrap();
            let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
            let maybe_server = server_map.get(uri);
            if let None = maybe_server {
                return Err(TransportError::new(format!(
                    "No Memory server at this url address: {}",
                    uri
                )));
            }
            println!("[t] (TransportMemory).send() {} | {}", uri, payload.len());
            // let uri = self.get_uri(id).unwrap();
            let mut server = maybe_server.unwrap().lock().unwrap();
            server
                .post(&self.maybe_my_uri.clone().unwrap().to_string(), payload)
                .expect("Post on memory server should work");
        }
        Ok(())
    }

    /// Send to all known transportIds
    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        let id_list = self.transport_id_list()?;
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
        memory_server::set_server(&bounded_uri)?;
        self.my_servers.insert(bounded_uri.clone());
        Ok(bounded_uri.clone())
    }

    /// Process my TransportCommand inbox and all my server inboxes
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        // println!("[t] (TransportMemory).process()");
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
        // Process my Servers
        let mut to_connect_list: Vec<Url> = Vec::new();
        for server_uri in &self.my_servers {
            let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
            let server = server_map.get(server_uri).expect("My server should exist.");
            let (success, output) = server.lock().unwrap().process()?;
            if success {
                did_work = true;
                for event in &output {
                    if let TransportEvent::ConnectResult(uri) = event {
                        let to_connect = Url::parse(uri).unwrap();
                        to_connect_list.push(to_connect);
                    } else {
                        outbox.push(event.clone());
                    }
                }
            }
        }
        // Connect back to received connections if not already connected to them
        for uri in to_connect_list {
            println!(
                "[t] (TransportMemory) {} <- {}",
                uri,
                self.maybe_my_uri.clone().unwrap()
            );
            if let Ok(id) = self.connect(&uri) {
                outbox.push(TransportEvent::ConnectResult(id));
            }
        }
        Ok((did_work, outbox))
    }
}

impl Drop for TransportMemory {
    fn drop(&mut self) {
        // Close all connections
        self.close_all().ok();
        // Drop my servers
        for bounded_url in &self.my_servers {
            memory_server::unset_server(&bounded_url)
                .expect("unset_server() during drop should never fail");
        }
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
        println!("[d] >>> '(TransportMemory)' recv cmd: {:?}", cmd);
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
                let evt = TransportEvent::Closed(id.to_string());
                Ok(vec![evt])
            }
            TransportCommand::CloseAll => {
                self.close_all()?;
                let mut outbox = Vec::new();
                for (id, _url) in &self.connections {
                    let evt = TransportEvent::Closed(id.to_string());
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