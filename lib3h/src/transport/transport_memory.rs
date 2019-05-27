use crate::transport::{
    error::TransportResult,
    memory_server,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    TransportId, TransportIdRef,
};
use lib3h_protocol::{DidWork, Lib3hResult};
use std::collections::{HashMap, HashSet, VecDeque};

/// Transport used for the Space P2pGateway
pub struct TransportMemory {
    // Commands sent to us by owner for async processing
    cmd_inbox: VecDeque<TransportCommand>,
    // Payloads sent tus by the network
    net_inbox_map: HashMap<TransportId, VecDeque<Vec<u8>>>,
    // url
    my_servers: HashSet<String>,
    connections: HashMap<TransportId, String>,
    n_id: u32,
}

impl TransportMemory {
    pub fn new() -> Self {
        TransportMemory {
            cmd_inbox: VecDeque::new(),
            net_inbox_map: HashMap::new(),
            my_servers: HashSet::new(),
            connections: HashMap::new(),
            n_id: 0,
        }
    }
}

/// Compose Transport
impl Transport for TransportMemory {
    fn transport_id_list(&self) -> TransportResult<Vec<TransportId>> {
        // connections.collect();
        // FIXME
        Ok(vec![])
    }

    fn connect(&mut self, uri: &str) -> TransportResult<TransportId> {
        let maybe_server = memory_server::get_server(url);
        if let None = maybe_server {
            return Err(format_err!("No Memory server at this url address: {}", uri));
        }
        self.n_id += 1;
        let id = format!("{}__{}", uri, self.n_id);
        maybe_server.unwrap().connect(id.clone());
        self.connections.insert(id, uri.to_string());
        Ok(id.clone())
    }

    fn close(&mut self, _id: TransportId) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    fn close_all(&mut self) -> TransportResult<()> {
        // FIXME
        Ok(())
    }

    fn send(&mut self, id_list: &[&TransportIdRef], _payload: &[u8]) -> TransportResult<()> {
        for id in id_list {
            let url = self.connections.get(id)?;
            let maybe_server = memory_server::get_server(url);
            if let None = maybe_server {
                return Err(format_err!("No Memory server at this url address: {}", url));
            }
            maybe_server
                .unwrap()
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
                outbox.append(output);
            }
        }
        // Process my Servers
        for server_url in my_servers {
            let server = memory_server::get_server(server_url).expect("My server should exist.");
            server.process()?;
        }
        Ok((did_work, outbox))
    }
}

impl TransportMemory {
    /// Process a transportEvent.
    /// Return a list of P2pProtocol messages to send to others.
    fn serve_TransportCommand(
        &mut self,
        cmd: &TransportCommand,
    ) -> Lib3hResult<(DidWork, Vec<TransportEvent>)> {
        println!("(log.d) >>> '(TransportMemory)' recv cmd: {:?}", cmd);
        // Note: use same order as the enum
        match cmd {
            TransportCommand::Connect(url) => {
                let id = self.connect(url)?;
                let evt = TransportEvent::ConnectResult(id);
                Ok((true, vec![evt]))
            }
            TransportCommand::Send(id_list, payload) => {
                let id = self.send(&id_list, payload)?;
                Ok((true, vec![]))
            }
            TransportCommand::SendAll(msg) => {
                let id = self.send_all(url)?;
                Ok((true, vec![]))
            }
            TransportCommand::Close(id) => {
                self.close(url)?;
                let evt = TransportEvent::Closed(id.to_string());
                Ok((true, vec![evt]))
            }
            TransportCommand::CloseAll => {
                self.close_all()?;
                let mut outbox = Vec::new();
                for id in connections {
                    let evt = TransportEvent::Closed(id);
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
