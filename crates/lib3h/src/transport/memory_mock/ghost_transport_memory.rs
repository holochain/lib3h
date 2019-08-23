use crate::transport::{
    error::TransportError, memory_mock::memory_server, protocol::*, ConnectionId,
};
use lib3h_ghost_actor::{GhostActor, GhostActorState, RequestId, WorkWasDone};
use std::{
    any::Any,
    collections::HashMap,
    sync::{Arc, Mutex},
};
use url::Url;

#[derive(Debug)]
#[allow(dead_code)]
enum RequestToParentContext {
    Source { address: Url },
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

                // get destinations server
                let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
                let maybe_server = server_map.get(&address);
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

                // Check if already connected
                let maybe_cid = self
                    .outbound_connection_map
                    .iter()
                    .find(|(_, cur_uri)| *cur_uri.to_string() == address.to_string());

                // if not
                if maybe_cid.is_none() {
                    // Generate and store a connectionId to act like other Transport types
                    self.n_id += 1;
                    let id = format!("mem_conn_{}_{}", self.own_id, self.n_id);
                    self.outbound_connection_map
                        .insert(id.clone(), address.clone());
                    // Connect to it
                    let result = server.request_connect(&my_addr, &id);
                    if result.is_err() {
                        self.respond_with(&request_id, RequestToChildResponse::SendMessage(result));
                        return;
                    }
                };

                trace!(
                    "(GhostTransportMemory).SendMessage from {} to  {} | {:?}",
                    my_addr,
                    address,
                    payload
                );
                // Send it data from us
                server
                    .post(&my_addr, &payload)
                    .expect("Post on memory server should work");

                self.respond_with(&request_id, RequestToChildResponse::SendMessage(Ok(())));
            }
        }
    }

    fn process_concrete(&mut self) -> Result<WorkWasDone, TransportError> {
        // make sure we have bound and get our address if so
        let my_addr = match &self.maybe_my_address {
            Some(my_addr) => my_addr.clone(),
            None => return Ok(false.into()),
        };

        println!("Processing for: {}", my_addr);

        // get our own server
        let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
        let maybe_server = server_map.get(&my_addr);
        if let None = maybe_server {
            return Err(TransportError::new(format!(
                "No Memory server at this address: {}",
                my_addr
            )));
        }
        let mut server = maybe_server.unwrap().lock().unwrap();
        let (success, event_list) = server.process()?;
        if success {
            let mut to_connect_list: Vec<(Url, ConnectionId)> = Vec::new();
            for event in event_list {
                match event {
                    TransportEvent::IncomingConnectionEstablished(in_cid) => {
                        let to_connect_uri = server
                            .get_inbound_uri(&in_cid)
                            .expect("Should always have uri");
                        to_connect_list.push((to_connect_uri.clone(), in_cid.clone()));
                        self.get_actor_state().send_event_to_parent(
                            RequestToParent::IncomingConnection {
                                address: to_connect_uri.clone(),
                            },
                        );
                    }
                    TransportEvent::ReceivedData(in_cid, payload) => {
                        println!("RecivedData--- cid:{:?} payload:{:?}", in_cid, payload);
                        let out_cid = self
                            .inbound_connection_map
                            .get(&in_cid)
                            .expect("Should have inbound at this stage")
                            .clone();
                        let out_addr = self
                            .outbound_connection_map
                            .get(&out_cid)
                            .expect("Should have outbound at this stage")
                            .clone();
                        self.get_actor_state().send_event_to_parent(
                            RequestToParent::ReceivedData {
                                address: out_addr.clone(),
                                payload,
                            },
                        );
                    }
                    _ => panic!(format!("WHAT: {:?}", event)), //                    output.push(event);
                };
            }

            // Connect back to received connections if not already connected to them
            for (uri, in_cid) in to_connect_list {
                println!("(GhostTransportMemory)connecting {} <- {:?}", uri, my_addr);
                // Check if already connected
                let maybe_cid = self
                    .outbound_connection_map
                    .iter()
                    .find(|(_, cur_uri)| *cur_uri.to_string() == my_addr.to_string());

                let out_cid = match maybe_cid {
                    None => {
                        // Generate and store a connectionId to act like other Transport types
                        self.n_id += 1;
                        let id = format!("mem_conn_{}_{}", self.own_id, self.n_id);
                        self.outbound_connection_map
                            .insert(id.clone(), my_addr.clone());
                        // Connect to it
                        let _result = server.request_connect(&my_addr, &id);
                        id
                    }
                    Some((id, _)) => id.to_string(),
                };
                self.inbound_connection_map
                    .insert(in_cid.clone(), out_cid.clone());
            }

            Ok(true.into())
        } else {
            Ok(false.into())
        }
    }
}

lazy_static! {
    /// Counter of the number of GhostTransportMemory that spawned
    static ref TRANSPORT_COUNT: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
}

#[cfg(test)]
mod tests {

    use super::*;
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

        // now send a message from transport1 to transport2 over the bound addresses
        let send_request1 = RequestId::with_prefix("test_parent");
        transport1.request(
            Some(send_request1),
            RequestToChild::SendMessage {
                address: Url::parse("mem://addr_2").unwrap(),
                payload: b"test message".to_vec(),
            },
        );

        // call process on both transports so queues can fill
        transport1.process().unwrap();
        transport2.process().unwrap();

        let mut r = transport2.drain_requests();
        let (_rid, request) = r.pop().unwrap();
        assert_eq!(
            "IncomingConnection { address: \"mem://addr_1/\" }",
            format!("{:?}", request)
        );

        let mut r = transport1.drain_responses();
        let (_rid, response) = r.pop().unwrap();
        assert_eq!("SendMessage(Ok(()))", format!("{:?}", response));

        // call process on memory_server now to get it to send
        // an incoming request to transport2
        transport1.process().unwrap();
        transport2.process().unwrap();

        let mut r = transport2.drain_requests();
        let (_rid, request) = r.pop().unwrap();
        assert_eq!("ReceivedData { address: \"mem://addr_1/\", payload: [116, 101, 115, 116, 32, 109, 101, 115, 115, 97, 103, 101] }", format!("{:?}", request));
    }
}
