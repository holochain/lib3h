use crate::transport::{
    error::{TransportError, TransportResult},
    memory_mock::memory_server,
    protocol::{TransportCommand, TransportEvent},
    transport_trait::Transport,
    ConnectionId, ConnectionIdRef,
};
use lib3h_protocol::DidWork;
use std::collections::{HashSet, VecDeque};
use url::Url;

/// Transport for mocking network layer in-memory
/// Binding creates a MemoryServer at url that can be accessed by other nodes
pub struct TransportMemory {
    /// Commands sent to us by owner for async processing
    cmd_inbox: VecDeque<TransportCommand>,
    /// Addresses (url-ish) of all our servers
    my_servers: HashSet<Url>,
    /// Addresses of connections to remotes
    connections: HashSet<Url>,
    /// The bound uri of my main server
    maybe_my_bound_uri: Option<Url>,
}

impl TransportMemory {
    pub fn new() -> Self {
        TransportMemory {
            cmd_inbox: VecDeque::new(),
            my_servers: HashSet::new(),
            connections: HashSet::new(),
            maybe_my_bound_uri: None,
        }
    }

    pub fn name(&self) -> &str {
        match &self.maybe_my_bound_uri {
            None => "",
            Some(uri) => uri.as_str(),
        }
    }

    ///
    pub fn is_connected_to(&self, uri_as_cid: &ConnectionIdRef) -> bool {
        match &self.maybe_my_bound_uri {
            None => false,
            Some(my_bound_uri) => {
                let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
                server_map
                    .get(my_bound_uri)
                    .map(|my_server| {
                        my_server
                            .lock()
                            .unwrap()
                            .is_connected_to(&Url::parse(uri_as_cid).unwrap())
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
        // Drop my servers
        for bounded_url in &self.my_servers {
            let _ = memory_server::unset_server(&bounded_url);
            //                .expect("unset_server() during drop should never fail");
        }
    }
}
/// Compose Transport
impl Transport for TransportMemory {
    /// Get list of known connectionIds
    fn connection_id_list(&mut self) -> TransportResult<Vec<ConnectionId>> {
        Ok(self.connections.iter().map(|uri| uri.to_string()).collect())
    }

    /// get uri from a connectionId
    fn get_uri(&self, uri_as_cid: &ConnectionIdRef) -> Option<Url> {
        let uri = Url::parse(uri_as_cid).expect("connectionId is not a valid Url");
        let res = self.connections.get(&uri);
        res.map(|url| url.clone()).or_else(|| {
            if self.is_connected_to(uri_as_cid) {
                match &self.maybe_my_bound_uri {
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
    fn connect(&mut self, remote_uri: &Url) -> TransportResult<ConnectionId> {
        // Check if already connected
        let maybe_uri = self.connections.get(remote_uri);
        if let Some(uri) = maybe_uri {
            return Ok(uri.to_string());
        }
        // Get my uri
        let my_uri = match &self.maybe_my_bound_uri {
            None => {
                return Err(TransportError::new(
                    "Must bind before attempting to connect".to_string(),
                ));
            }
            Some(u) => u,
        };
        // Get other node's server
        let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
        let maybe_remote_server = server_map.get(remote_uri);
        if let None = maybe_remote_server {
            return Err(TransportError::new(format!(
                "No Memory server at this url address: {}",
                remote_uri
            )));
        }
        // Connect to it
        let mut remote_server = maybe_remote_server.unwrap().lock().unwrap();
        remote_server.request_connect(my_uri)?;
        self.connections.insert(remote_uri.clone());
        Ok(remote_uri.to_string())
    }

    /// Notify remote server on that connectionId that we are closing connection and
    /// locally clear that connectionId.
    fn close(&mut self, uri_as_cid: &ConnectionIdRef) -> TransportResult<()> {
        let remote_uri = Url::parse(uri_as_cid).expect("connectionId is not a valid Url");
        trace!("TransportMemory.close({})", remote_uri.path());
        if self.maybe_my_bound_uri.is_none() {
            return Err(TransportError::new(
                "Cannot close a connection before binding".to_string(),
            ));
        }
        let my_uri = self.maybe_my_bound_uri.clone().unwrap();
        // Check if we are connected to that uri
        if !self.connections.contains(&remote_uri) {
            return Err(TransportError::new(format!(
                "Unknown connectionId: {}",
                uri_as_cid
            )));
        }
        // Get other node's server
        let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
        let maybe_remote_server = server_map.get(&remote_uri);
        if let None = maybe_remote_server {
            return Err(TransportError::new(format!(
                "No Memory server at this url: {}",
                remote_uri,
            )));
        }
        let mut remote_server = maybe_remote_server.unwrap().lock().unwrap();
        // Tell it we closed connection with it
        let _ = remote_server.request_close(&my_uri);
        // Locally remove connection
        self.connections.remove(&remote_uri);
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
    fn send(
        &mut self,
        uri_as_cid_list: &[&ConnectionIdRef],
        payload: &[u8],
    ) -> TransportResult<()> {
        if self.maybe_my_bound_uri.is_none() {
            return Err(TransportError::new(
                "Cannot send before bounding".to_string(),
            ));
        }
        let my_uri = self.maybe_my_bound_uri.clone().unwrap();
        for uri_as_cid in uri_as_cid_list {
            // Get the other node's uri on that connection
            let remote_uri = Url::parse(uri_as_cid).expect("connectionId is not a valid Url");
            if !self.connections.contains(&remote_uri) {
                warn!("No known connection for: {}", uri_as_cid);
                continue;
            }
            // Get the other node's server
            let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
            let maybe_remote_server = server_map.get(&remote_uri);
            if let None = maybe_remote_server {
                return Err(TransportError::new(format!(
                    "No Memory server at this url address: {}",
                    remote_uri
                )));
            }
            trace!(
                "(TransportMemory).send() {} | {}",
                remote_uri,
                payload.len()
            );
            let mut remote_server = maybe_remote_server.unwrap().lock().unwrap();
            // Send it data from us
            remote_server
                .post(&my_uri, payload)
                .expect("Post on memory server should work");
        }
        Ok(())
    }

    /// Send to all known connectionIds
    fn send_all(&mut self, payload: &[u8]) -> TransportResult<()> {
        let cid_list = self.connection_id_list()?;
        for cid in cid_list {
            self.send(&[cid.as_str()], payload)?;
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
        let bound_uri = Url::parse(format!("{}_bound", uri).as_str()).unwrap();
        self.maybe_my_bound_uri = Some(bound_uri.clone());
        memory_server::set_server(&bound_uri)?;
        self.my_servers.insert(bound_uri.clone());
        Ok(bound_uri.clone())
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
                    if let TransportEvent::IncomingConnectionEstablished(in_cid) = event {
                        let to_connect_uri =
                            Url::parse(&in_cid).expect("connectionId is not a valid Url");
                        to_connect_list.push(to_connect_uri.clone());
                    } else {
                        output.push(event);
                    }
                }
            }
        }
        // Connect back to received connections if not already connected to them
        for in_uri in to_connect_list {
            trace!(
                "(TransportMemory) {} <- {:?}",
                in_uri,
                self.maybe_my_bound_uri
            );
            let cid = self.connect(&in_uri)?;
            // Note: Add IncomingConnectionEstablished events at start of outbox
            // so they can be processed first.
            outbox.insert(0, TransportEvent::IncomingConnectionEstablished(cid));
        }
        // process other messages
        for event in output {
            match event {
                TransportEvent::ConnectionClosed(in_cid) => {
                    // close will fail as other side isn't there anymore
                    let _ = self.close(&in_cid);
                    outbox.push(TransportEvent::ConnectionClosed(in_cid.to_string()));
                }
                TransportEvent::ReceivedData(in_cid, data) => {
                    outbox.push(TransportEvent::ReceivedData(
                        in_cid.to_string(),
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
            TransportCommand::Connect(remote_uri, request_id) => {
                let remote_uri_as_cid = self.connect(remote_uri)?;
                let evt = TransportEvent::ConnectResult(remote_uri_as_cid, request_id.clone());
                Ok(vec![evt])
            }
            TransportCommand::Send(uri_as_cid_list, payload) => {
                let mut id_ref_list = Vec::with_capacity(uri_as_cid_list.len());
                for id in uri_as_cid_list {
                    id_ref_list.push(id.as_str());
                }
                let _id = self.send(&id_ref_list, payload)?;
                Ok(vec![])
            }
            TransportCommand::SendAll(payload) => {
                let _id = self.send_all(payload)?;
                Ok(vec![])
            }
            TransportCommand::Close(uri_as_cid) => {
                self.close(uri_as_cid)?;
                let evt = TransportEvent::ConnectionClosed(uri_as_cid.to_string());
                Ok(vec![evt])
            }
            TransportCommand::CloseAll => {
                self.close_all()?;
                let mut outbox = Vec::new();
                for remote_uri in &self.connections {
                    let evt = TransportEvent::ConnectionClosed(remote_uri.to_string());
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
