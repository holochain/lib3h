use crate::transport::{
    error::{TransportError, TransportResult},
    memory_mock::memory_server,
    protocol::*,
    transport_trait::Transport,
};
use lib3h_protocol::DidWork;
use std::collections::{HashSet, VecDeque};
use url::Url;
/// Transport for mocking network layer in-memory
/// Binding creates a MemoryServer at url that can be accessed by other nodes
pub struct TransportMemory {
    /// Commands sent to us by owner for async processing
    cmd_inbox: VecDeque<TransportCommand>,
    /// Messages waiting send on process
    evt_outbox: Vec<TransportEvent>,
    /// Addresses (url-ish) of all our servers
    my_servers: HashSet<Url>,
    /// Addresses of connections to remotes
    connections: HashSet<Url>,
    /// My peer uri on the network layer
    maybe_my_uri: Option<Url>,
}

impl TransportMemory {
    pub fn new() -> Self {
        TransportMemory {
            cmd_inbox: VecDeque::new(),
            evt_outbox: Vec::new(),
            my_servers: HashSet::new(),
            connections: HashSet::new(),
            maybe_my_uri: None,
        }
    }

    pub fn name(&self) -> &str {
        match &self.maybe_my_uri {
            None => "",
            Some(uri) => uri.as_str(),
        }
    }

    pub fn is_bound(&self, address: &Url) -> bool {
        match &self.maybe_my_uri {
            None => false,
            Some(uri) => {
                let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
                server_map
                    .get(uri)
                    .map(|server| server.lock().unwrap().has_inbound_address(address))
                    .unwrap_or(false)
            }
        }
    }

    /// Create a new server inbox for myself
    fn priv_bind(&mut self, request_id: RequestId, spec: Url) -> TransportResult<()> {
        let bound_address = Url::parse(format!("{}/bound", spec).as_str()).expect("can parse url");
        self.maybe_my_uri = Some(bound_address.clone());
        memory_server::set_server(&bound_address)?;
        self.my_servers.insert(bound_address.clone());
        self.evt_outbox.push(TransportEvent::BindSuccess {
            request_id,
            bound_address,
        });
        Ok(())
    }

    /// Connect to another node's "bind".
    /// Get server from the uri and connect to it with a new connectionId for ourself.
    fn priv_connect(&mut self, request_id: RequestId, address: Url) -> TransportResult<()> {
        if self.connections.contains(&address) {
            self.evt_outbox.push(TransportEvent::ConnectSuccess {
                request_id,
                address,
            });
            return Ok(());
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
        let maybe_server = server_map.get(&address);
        if let None = maybe_server {
            return Err(TransportError::new(format!(
                "No Memory server at this url address: {}",
                address
            )));
        }
        self.connections.insert(address.clone());
        // Connect to it
        let mut server = maybe_server.unwrap().lock().unwrap();
        server.request_connect(my_uri)?;

        self.evt_outbox.push(TransportEvent::ConnectSuccess {
            request_id,
            address,
        });

        Ok(())
    }

    /// Send payload to known connectionIds in `id_list`
    fn priv_send(
        &mut self,
        request_id: RequestId,
        address: Url,
        payload: Vec<u8>,
    ) -> TransportResult<()> {
        if self.maybe_my_uri.is_none() {
            return Err(TransportError::new(
                "Cannot send before binding".to_string(),
            ));
        }

        let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
        let maybe_server = server_map.get(&address);
        if let None = maybe_server {
            return Err(TransportError::new(format!(
                "No Memory server at this url address: {}",
                address
            )));
        }
        trace!("(TransportMemory).send() {} | {}", address, payload.len());
        let mut server = maybe_server.unwrap().lock().unwrap();
        // Send it data from us
        server
            .post(&address, &payload)
            .expect("Post on memory server should work");

        self.evt_outbox
            .push(TransportEvent::SendMessageSuccess { request_id });

        Ok(())
    }
}

impl Drop for TransportMemory {
    fn drop(&mut self) {
        // Close all connections
        //self.close_all().ok();
        // Drop my servers
        for bounded_url in &self.my_servers {
            memory_server::unset_server(&bounded_url)
                .expect("unset_server() during drop should never fail");
        }
    }
}
/// Compose Transport
impl Transport for TransportMemory {
    /// Get list of known connectionIds
    fn connection_list(&self) -> TransportResult<Vec<Url>> {
        Ok(self.my_servers.iter().map(|x| x.clone()).collect())
    }

    /// Add Command to inbox
    fn post(&mut self, command: TransportCommand) -> TransportResult<()> {
        self.cmd_inbox.push_back(command);
        Ok(())
    }

    /// Process my TransportCommand inbox and all my server inboxes
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        // trace!("(TransportMemory).process()");
        let mut did_work = false;
        // Process TransportCommand inbox
        loop {
            let cmd = match self.cmd_inbox.pop_front() {
                None => break,
                Some(msg) => msg,
            };
            did_work = true;
            self.serve_TransportCommand(cmd)?;
        }
        // Process my Servers: process IncomingConnectionEstablished first
        let mut to_connect_list: Vec<Url> = Vec::new();
        let mut output = Vec::new();
        for my_server_uri in &self.my_servers {
            let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
            let mut my_server = server_map
                .get(my_server_uri)
                .expect("My server should exist.")
                .lock()
                .unwrap();
            let (success, event_list) = my_server.process()?;
            if success {
                did_work = true;

                for event in event_list {
                    if let TransportEvent::IncomingConnection { address } = event {
                        to_connect_list.push(address);
                    } else {
                        output.push(event);
                    }
                }
            }
        }
        // Connect back to received connections if not already connected to them
        for address in to_connect_list {
            trace!("(TransportMemory) {} <- {:?}", address, self.maybe_my_uri);
            self.connect("".to_string(), address.clone())?;
            self.connections.insert(address.clone());
            // Note: Add IncomingConnectionEstablished events at start of outbox
            // so they can be processed first.
            self.evt_outbox
                .insert(0, TransportEvent::IncomingConnection { address });
        }
        // process other messages
        for event in output {
            match event {
                TransportEvent::ConnectionClosed { address } => {
                    // convert inbound connectionId to outbound connectionId.
                    // let out_cid = self.inbound_connection_map.get(&in_cid).expect("Should have outbound at this stage");
                    self.connections.remove(&address);
                    self.evt_outbox
                        .push(TransportEvent::ConnectionClosed { address });
                }
                TransportEvent::ReceivedData { address, payload } => {
                    self.evt_outbox
                        .push(TransportEvent::ReceivedData { address, payload });
                }
                // We are not expecting anything else from the MemoryServer
                _ => unreachable!(),
            }
        }
        // Done
        Ok((did_work, self.evt_outbox.drain(..).collect()))
    }

    fn bind_sync(&mut self, spec: Url) -> TransportResult<Url> {
        let rid = nanoid::simple();
        self.bind(rid.clone(), spec)?;
        for _x in 0..100 {
            let (_, evt_list) = self.process()?;
            let mut out = None;
            for evt in evt_list {
                match &evt {
                    TransportEvent::BindSuccess {
                        request_id,
                        bound_address,
                    } => {
                        if request_id == &rid {
                            out = Some(bound_address.clone());
                        }
                        self.evt_outbox.push(evt);
                    }
                    _ => self.evt_outbox.push(evt),
                }
            }
            if out.is_some() {
                return Ok(out.unwrap());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Err(TransportError::new("bind fail".into()))
    }
}

impl TransportMemory {
    /// Process a TransportCommand: Call the corresponding method and possibily return some Events.
    /// Return a list of TransportEvents to owner.
    #[allow(non_snake_case)]
    fn serve_TransportCommand(&mut self, cmd: TransportCommand) -> TransportResult<()> {
        debug!(">>> '(TransportMemory)' recv cmd: {:?}", cmd);
        // Note: use same order as the enum
        match cmd {
            TransportCommand::Bind { request_id, spec } => self.priv_bind(request_id, spec),
            TransportCommand::Connect {
                request_id,
                address,
            } => self.priv_connect(request_id, address),
            TransportCommand::SendMessage {
                request_id,
                address,
                payload,
            } => self.priv_send(request_id, address, payload),
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
        assert!(transport.bind_sync(bind_url.clone()).is_ok());
        assert!(transport.bind_sync(bind_url).is_ok());
    }

}
