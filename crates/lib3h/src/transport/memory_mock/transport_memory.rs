use crate::transport::{
    error::{TransportError, TransportResult},
    memory_mock::memory_server,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    TransportId, TransportIdRef,
};
use lib3h_protocol::DidWork;
use std::collections::{HashMap, HashSet, VecDeque};

/// Transport used for the Space P2pGateway
pub struct TransportMemory {
    /// Commands sent to us by owner for async processing
    cmd_inbox: VecDeque<TransportCommand>,
    /// All of our servers (by url)
    my_servers: HashSet<String>,
    /// Mapping of transportId -> serverUrl
    connections: HashMap<TransportId, String>,
    /// Counter for generating new transportIds
    n_id: u32,
}

impl TransportMemory {
    pub fn new() -> Self {
        TransportMemory {
            cmd_inbox: VecDeque::new(),
            my_servers: HashSet::new(),
            connections: HashMap::new(),
            n_id: 0,
        }
    }
}

/// Compose Transport
impl Transport for TransportMemory {
    /// Get list of known transportIds
    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        Ok(self.connections.keys().map(|id| id.to_string()).collect())
    }

    /// Get server from the uri and connect to it with a new transportId for ourself.
    fn connect(&mut self, uri: &str) -> TransportResult<TransportId> {
        // println!("[d] ---- connect: {}", uri);
        let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
        let maybe_server = server_map.get(uri);
        if let None = maybe_server {
            return Err(TransportError::new(format!(
                "No Memory server at this url address: {}",
                uri
            )));
        }
        let mut server = maybe_server.unwrap().lock().unwrap();
        self.n_id += 1;
        let id = format!("{}__{}", uri, self.n_id);
        server.connect(&id)?;
        self.connections.insert(id.clone(), uri.to_string());
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
        let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
        let maybe_server = server_map.get(url);
        if let None = maybe_server {
            return Err(TransportError::new(format!(
                "No Memory server at this url: {}",
                url,
            )));
        }
        let mut server = maybe_server.unwrap().lock().unwrap();
        server.close(&id)?;
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
            let maybe_url = self.connections.get(*id);
            if let None = maybe_url {
                println!("[w] No known connection for TransportId {}", id);
                continue;
            }
            let url = maybe_url.unwrap();
            let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
            let maybe_server = server_map.get(url);
            if let None = maybe_server {
                return Err(TransportError::new(format!(
                    "No Memory server at this url address: {}",
                    url
                )));
            }
            let mut server = maybe_server.unwrap().lock().unwrap();
            server
                .post(id, payload)
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
    fn bind(&mut self, url: &str) -> TransportResult<String> {
        memory_server::set_server(url)?;
        self.my_servers.insert(url.to_string());
        Ok(url.to_string())
    }

    /// Process my TransportCommand inbox and all my server inboxes
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
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
        for server_url in &self.my_servers {
            let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
            let server = server_map.get(server_url).expect("My server should exist.");
            let (success, mut output) = server.lock().unwrap().process()?;
            if success {
                did_work = true;
                outbox.append(&mut output);
            }
        }
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
