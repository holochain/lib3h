use crate::transport::{
    error::{TransportError, TransportResult},
    memory_server,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    TransportId, TransportIdRef,
};
use lib3h_protocol::DidWork;
use std::collections::{HashMap, HashSet, VecDeque};

/// Transport used for the Space P2pGateway
pub struct TransportMemory {
    // Commands sent to us by owner for async processing
    cmd_inbox: VecDeque<TransportCommand>,
    // url
    my_servers: HashSet<String>,
    connections: HashMap<TransportId, String>,
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
    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        Ok(self.connections.keys().map(|id| id.to_string()).collect())
    }

    fn connect(&mut self, uri: &str) -> TransportResult<TransportId> {
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

    fn close(&mut self, _id: &TransportIdRef) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    fn close_all(&mut self) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    fn send(&mut self, id_list: &[&TransportIdRef], payload: &[u8]) -> TransportResult<()> {
        for id in id_list {
            let maybe_url = self.connections.get(id.clone());
            if let None = maybe_url {
                println!("[w] No Connection for TransportId {}", id);
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

    fn send_all(&mut self, _payload: &[u8]) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.cmd_inbox.push_back(command);
        Ok(())
    }

    fn bind(&mut self, url: &str) -> TransportResult<()> {
        memory_server::set_server(url)?;
        self.my_servers.insert(url.to_string());
        Ok(())
    }

    // FIXME
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        let mut outbox = Vec::new();
        let mut did_work = false;
        // Process Commands
        loop {
            let cmd = match self.cmd_inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            let (success, mut output) = self.serve_TransportCommand(&cmd)?;
            if success {
                did_work = true;
                outbox.append(&mut output);
            }
        }
        // Process my Servers
        for server_url in &self.my_servers {
            let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
            let server = server_map.get(server_url).expect("My server should exist.");
            server.lock().unwrap().process()?;
        }
        Ok((did_work, outbox))
    }
}

impl TransportMemory {
    /// Process a transportEvent.
    /// Return a list of P2pProtocol messages to send to others.
    #[allow(non_snake_case)]
    fn serve_TransportCommand(
        &mut self,
        cmd: &TransportCommand,
    ) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        println!("(log.d) >>> '(TransportMemory)' recv cmd: {:?}", cmd);
        // Note: use same order as the enum
        match cmd {
            TransportCommand::Connect(url) => {
                let id = self.connect(url)?;
                let evt = TransportEvent::ConnectResult(id);
                Ok((true, vec![evt]))
            }
            TransportCommand::Send(id_list, payload) => {
                let mut id_ref_list = Vec::with_capacity(id_list.len());
                for id in id_list {
                    id_ref_list.push(id.as_str());
                }
                // let id_list_ref = id_list.as_slice().as_ref();
                let _id = self.send(&id_ref_list, payload)?;
                Ok((true, vec![]))
            }
            TransportCommand::SendAll(payload) => {
                let _id = self.send_all(payload)?;
                Ok((true, vec![]))
            }
            TransportCommand::Close(id) => {
                self.close(id)?;
                let evt = TransportEvent::Closed(id.to_string());
                Ok((true, vec![evt]))
            }
            TransportCommand::CloseAll => {
                self.close_all()?;
                let mut outbox = Vec::new();
                for (id, _url) in &self.connections {
                    let evt = TransportEvent::Closed(id.to_string());
                    outbox.push(evt);
                }
                Ok((true, outbox))
            }
            TransportCommand::Bind(url) => {
                self.bind(url)?;
                Ok((true, vec![]))
            }
        }
    }
}
