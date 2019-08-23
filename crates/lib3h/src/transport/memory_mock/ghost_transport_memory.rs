use crate::transport::{
    error::{TransportError, TransportResult},
    memory_mock::memory_server,
    protocol::*,
    transport_trait::Transport,
    ConnectionId, ConnectionIdRef,
};
use lib3h_ghost_actor::{GhostActor, GhostActorState, RequestId, WorkWasDone};
use lib3h_protocol::DidWork;
use std::{
    any::Any,
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
};
use url::Url;
/// Transport for mocking network layer in-memory
/// Binding creates a MemoryServer at url that can be accessed by other nodes
pub struct OldTransportMemory {
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

#[derive(Debug)]
#[allow(dead_code)]
enum RequestToParentContext {
    IncomingConnection { address: String },
}

type GhostTransportMemoryState = GhostActorState<
    RequestToParentContext,
    RequestToParent,
    RequestToParentResponse,
    RequestToChildResponse,
    TransportError,
>;

#[allow(dead_code)]
struct GhostTransportMemory {
    actor_state: Option<GhostTransportMemoryState>,
    /// My peer uri on the network layer (not None after a bind)
    maybe_my_address: Option<Url>,
    /// Mapping of connectionId -> serverUrl
    outbound_connection_map: HashMap<ConnectionId, Url>,
    /// Mapping of in:connectionId -> out:connectionId
    inbound_connection_map: HashMap<ConnectionId, ConnectionId>,
    /// Counter for generating new connectionIds
    n_id: u32,
    own_id: u32,
    //    dht: DhtActor,
    //    dht_callbacks: Option<GhostTracker<GwDht, dht_protocol::RequestToChildResponse, String>>,
}

impl GhostTransportMemory {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let mut tc = TRANSPORT_COUNT
            .lock()
            .expect("could not lock transport count mutex");
        *tc += 1;
        Self {
            actor_state: Some(GhostActorState::new()),
            maybe_my_address: None,
            outbound_connection_map: HashMap::new(),
            inbound_connection_map: HashMap::new(),
            n_id: 0,
            own_id: *tc,
            //            dht: Box::new(RrDht::new()),
            //            dht_callbacks: Some(GhostTracker::new("gateway_transport_dht_")),
        }
    }

    fn respond_with(&mut self, request_id: &Option<RequestId>, response: RequestToChildResponse) {
        if let Some(request_id) = request_id {
            self.get_actor_state()
                .respond_to_parent(request_id.clone(), response);
        }
    }
}

macro_rules! is_bound {
    ($self:ident, $request_id:ident, $response_type:ident  ) => {
        match &mut $self.maybe_my_address {
            Some(my_addr) => my_addr.clone(),
            None => {
                $self.respond_with(
                    &$request_id,
                    RequestToChildResponse::$response_type(Err(TransportError::new(
                        "Transport must be bound before sending".to_string(),
                    ))),
                );
                return;
            }
        }
    };
}

/*
macro_rules! with_server {
    ($self:ident, $request_id:ident, $response_type:ident, $address:ident, |$server:ident| $code:expr  ) => {
        let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
        let maybe_server = server_map.get(&$address);
        if let None = maybe_server {
            respond_with!($self,$request_id,$response_type,
                          Err(TransportError::new(format!(
                              "No Memory server at this url address: {}",
                              $address
                          ))));
            return;
        }
        let mut server = maybe_server.unwrap().lock().unwrap();
        $code
    }
}
 */

impl
    GhostActor<
        RequestToParentContext,
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        TransportError,
    > for GhostTransportMemory
{
    // BOILERPLATE START----------------------------------

    fn as_any(&mut self) -> &mut dyn Any {
        &mut *self
    }

    fn get_actor_state(&mut self) -> &mut GhostTransportMemoryState {
        self.actor_state.as_mut().unwrap()
    }

    fn take_actor_state(&mut self) -> GhostTransportMemoryState {
        std::mem::replace(&mut self.actor_state, None).unwrap()
    }

    fn put_actor_state(&mut self, actor_state: GhostTransportMemoryState) {
        std::mem::replace(&mut self.actor_state, Some(actor_state));
    }

    // BOILERPLATE END----------------------------------

    // our parent is making a request of us
    //#[allow(irrefutable_let_patterns)]
    fn request(&mut self, request_id: Option<RequestId>, request: RequestToChild) {
        match request {
            RequestToChild::Bind { spec: _url } => {
                // get a new bound url from the memory server (we ignore the spec here)
                let bound_url = memory_server::new_url();
                memory_server::set_server(&bound_url).unwrap(); //set_server always returns Ok
                self.maybe_my_address = Some(bound_url.clone());

                // respond to our parent
                self.respond_with(
                    &request_id,
                    RequestToChildResponse::Bind(Ok(BindResultData {
                        bound_url: bound_url,
                    })),
                );
            }
            RequestToChild::SendMessage { address, payload } => {
                // make sure we have bound and get our address if so
                let my_addr = is_bound!(self, request_id, SendMessage);

                let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
                let maybe_server = server_map.get(&my_addr);
                if let None = maybe_server {
                    self.respond_with(
                        &request_id,
                        RequestToChildResponse::SendMessage(Err(TransportError::new(format!(
                            "No Memory server at this address: {}",
                            my_addr
                        )))),
                    );
                    return;
                }
                let mut server = maybe_server.unwrap().lock().unwrap();

                trace!(
                    "(GhostTransportMemory).send() {} | {}",
                    address,
                    payload.len()
                );
                // Send it data from us
                server
                    .post(&my_addr, &payload)
                    .expect("Post on memory server should work");

                self.respond_with(&request_id, RequestToChildResponse::SendMessage(Ok(())));
            }
            RequestToChild::Bootstrap { address } => {
                // make sure we have bound and get our address if so
                let my_addr = is_bound!(self, request_id, Bootstrap);

                // Get the other node's server
                // TODO: convert to macro:
                //    let server = get_server!(self,request_id,Bootstrap)
                let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
                let maybe_server = server_map.get(&address);
                if let None = maybe_server {
                    self.respond_with(
                        &request_id,
                        RequestToChildResponse::Bootstrap(Err(TransportError::new(format!(
                            "No Memory server at this address: {}",
                            my_addr
                        )))),
                    );

                    return;
                }
                let mut server = maybe_server.unwrap().lock().unwrap();

                // Check if already connected
                let maybe_cid = self
                    .outbound_connection_map
                    .iter()
                    .find(|(_, cur_uri)| *cur_uri.to_string() == address.to_string());

                if maybe_cid.is_none() {
                    // Generate and store a connectionId to act like other Transport types
                    self.n_id += 1;
                    let id = format!("mem_conn_{}_{}", self.own_id, self.n_id);
                    self.outbound_connection_map
                        .insert(id.clone(), address.clone());
                    // Connect to it
                    let result = server.request_connect(&my_addr, &id);
                    if result.is_err() {
                        self.respond_with(&request_id, RequestToChildResponse::Bootstrap(result));
                        return;
                    }
                };

                self.respond_with(&request_id, RequestToChildResponse::Bootstrap(Ok(())));
            }
        }
    }

    fn process_concrete(&mut self) -> Result<WorkWasDone, TransportError> {
        Ok(false.into())
    }
}

lazy_static! {
    /// Counter of the number of GhostTransportMemory that spawned
    static ref TRANSPORT_COUNT: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
}

impl OldTransportMemory {
    pub fn new() -> Self {
        let mut tc = TRANSPORT_COUNT
            .lock()
            .expect("could not lock transport count mutex");
        *tc += 1;
        OldTransportMemory {
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
                    .map(|server| server.lock().unwrap().get_inbound_uri(id).is_some())
                    .unwrap_or(false)
            }
        }
    }
}

impl Drop for OldTransportMemory {
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
/// Compose Transport
impl Transport for OldTransportMemory {
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
        let mut server = maybe_server.unwrap().lock().unwrap();
        server.request_connect(my_uri, &id)?;
        Ok(id)
    }

    /// Notify other server on that connectionId that we are closing connection and
    /// locally clear that connectionId.
    fn close(&mut self, id: &ConnectionIdRef) -> TransportResult<()> {
        trace!("OldTransportMemory[{}].close({})", self.own_id, id);
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
        let mut other_server = maybe_other_server.unwrap().lock().unwrap();
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
            trace!("(OldTransportMemory).send() {} | {}", uri, payload.len());
            let mut server = maybe_server.unwrap().lock().unwrap();
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
        memory_server::set_server(&bounded_uri)?;
        self.my_servers.insert(bounded_uri.clone());
        Ok(bounded_uri.clone())
    }

    /// Process my TransportCommand inbox and all my server inboxes
    fn process(&mut self) -> TransportResult<(DidWork, Vec<TransportEvent>)> {
        // trace!("(OldTransportMemory).process()");
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
            trace!("(OldTransportMemory) {} <- {:?}", uri, self.maybe_my_uri);
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

impl OldTransportMemory {
    /// Process a TransportCommand: Call the corresponding method and possibily return some Events.
    /// Return a list of TransportEvents to owner.
    #[allow(non_snake_case)]
    fn serve_TransportCommand(
        &mut self,
        cmd: &TransportCommand,
    ) -> TransportResult<Vec<TransportEvent>> {
        debug!(">>> '(OldTransportMemory)' recv cmd: {:?}", cmd);
        // Note: use same order as the enum
        match cmd {
            TransportCommand::Connect(url, request_id) => {
                let id = self.connect(url)?;
                let evt = TransportEvent::ConnectResult(id, request_id.clone());
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
        let mut transport = OldTransportMemory::new();
        let bind_url = url::Url::parse("mem://can_rebind").unwrap();
        assert!(transport.bind(&bind_url).is_ok());
        assert!(transport.bind(&bind_url).is_ok());
    }

    use lib3h_ghost_actor::RequestId;
    // use protocol::RequestToChildResponse;

    #[test]
    fn test_gmem_transport() {
        let mut transport1 = GhostTransportMemory::new();
        let mut transport2 = GhostTransportMemory::new();

        // create two memory bindings so that we have addresses
        let bind_request1 = RequestId::with_prefix("test_parent");
        let bind_request2 = RequestId::with_prefix("test_parent");

        assert_eq!(transport1.maybe_my_address, None);
        assert_eq!(transport2.maybe_my_address, None);

        transport1.request(
            Some(bind_request1),
            RequestToChild::Bind {
                spec: Url::parse("mem://_").unwrap(),
            },
        );
        transport2.request(
            Some(bind_request2),
            RequestToChild::Bind {
                spec: Url::parse("mem://_").unwrap(),
            },
        );

        let expected_transport1_address = Url::parse("mem://addr_1").unwrap();
        assert_eq!(
            transport1.maybe_my_address,
            Some(expected_transport1_address.clone())
        );
        let mut r1 = transport1.drain_responses();
        let (_rid, response) = r1.pop().unwrap();
        match response {
            RequestToChildResponse::Bind(Ok(bind_result)) => {
                // the memory transport server should bind us to the first available url which is a1
                assert_eq!(bind_result.bound_url, expected_transport1_address);
            }
            _ => assert!(false),
        }

        let expected_transport2_address = Url::parse("mem://addr_2").unwrap();
        assert_eq!(
            transport2.maybe_my_address,
            Some(expected_transport2_address.clone())
        );
        let mut r2 = transport2.drain_responses();
        let (_rid, response) = r2.pop().unwrap();
        match response {
            RequestToChildResponse::Bind(Ok(bind_result)) => {
                // the memory transport server should bind us to the first available url which is a1
                assert_eq!(bind_result.bound_url, expected_transport2_address);
            }
            _ => assert!(false),
        }

        // now bootstrap a connection between transport 1 & 2
        let bootstrap_request1 = RequestId::with_prefix("test_parent");
        transport1.request(
            Some(bootstrap_request1),
            RequestToChild::Bootstrap {
                address: Url::parse("mem://addr_2").unwrap(),
            },
        );

        let mut r = transport1.drain_responses();
        let (_rid, response) = r.pop().unwrap();
        assert_eq!("Bootstrap(Ok(()))", format!("{:?}", response));

        // call process on both transports so queues can fill
        transport1.process().unwrap();
        transport2.process().unwrap();

        let mut r = transport2.drain_requests();
        let (_rid, request) = r.pop().unwrap();
        assert_eq!("fish2", format!("{:?}", request));

        // now send a message from transport1 to transport2 over the bound addresses
        let send_request1 = RequestId::with_prefix("test_parent");
        transport1.request(
            Some(send_request1),
            RequestToChild::SendMessage {
                address: Url::parse("mem://addr_2").unwrap(),
                payload: b"test message".to_vec(),
            },
        );

        let mut r = transport1.drain_responses();
        let (_rid, response) = r.pop().unwrap();
        assert_eq!("SendMessage(Ok(()))", format!("{:?}", response));

        // TODO: need to call process on memory_server now to get it to send
        // an incoming request to transport2
        let mut r = transport2.drain_requests();
        let (_rid, request) = r.pop().unwrap();
        assert_eq!("fish2", format!("{:?}", request));
    }
}
