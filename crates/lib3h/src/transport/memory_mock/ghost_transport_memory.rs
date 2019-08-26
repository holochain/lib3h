use crate::transport::{error::TransportError, memory_mock::memory_server, protocol::*};
use lib3h_ghost_actor::{
    create_ghost_channel, GhostActor, GhostChannel, GhostContextChannel, GhostError, GhostResult,
    WorkWasDone,
};
use std::{
    any::Any,
    collections::HashSet,
    sync::{Arc, Mutex},
};
use url::Url;

#[derive(Debug)]
#[allow(dead_code)]
enum RequestToParentContext {
    Source { address: Url },
}

type GhostTransportMemoryChannel = GhostChannel<
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    TransportError,
>;

type GhostTransportMemoryChannelContext = GhostContextChannel<
    Url,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    TransportError,
>;

#[allow(dead_code)]
struct GhostTransportMemory {
    channel_parent: Option<GhostTransportMemoryChannel>,
    channel_self: Option<GhostTransportMemoryChannelContext>,
    /// My peer uri on the network layer (not None after a bind)
    maybe_my_address: Option<Url>,
    /// Addresses of connections to remotes
    connections: HashSet<Url>,
}

impl GhostTransportMemory {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let (channel_parent, channel_self) = create_ghost_channel();
        Self {
            channel_parent: Some(channel_parent),
            channel_self: Some(channel_self.as_context_channel()),
            connections: HashSet::new(),
            maybe_my_address: None,
        }
    }
}

/*
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
*/
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

impl From<TransportError> for GhostError {
    fn from(e: TransportError) -> Self {
        format!("TransportError: {}", e).into()
    }
}

impl
    GhostActor<
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

    fn take_parent_channel(&mut self) -> Option<GhostTransportMemoryChannel> {
        std::mem::replace(&mut self.channel_parent, None)
    }

    // BOILERPLATE END----------------------------------

    /*
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

                // if not already connected, request a connections
                if self.connections.get(&address).is_none() {
                    let result = server.request_connect(&my_addr);
                    if result.is_err() {
                        self.respond_with(&request_id, RequestToChildResponse::SendMessage(result));
                        return;
                    }
                    self.connections.insert(address.clone());
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
     */

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        // process the self channel
        let mut channel_self = std::mem::replace(&mut self.channel_self, None);
        channel_self
            .as_mut()
            .expect("exists")
            .process(self.as_any())?;
        std::mem::replace(&mut self.channel_self, channel_self);

        for mut msg in self.channel_self.as_mut().expect("exists").drain_requests() {
            match msg.take_payload().expect("exists") {
                RequestToChild::Bind { spec: _url } => {
                    // get a new bound url from the memory server (we ignore the spec here)
                    let bound_url = memory_server::new_url();
                    memory_server::set_server(&bound_url).unwrap(); //set_server always returns Ok
                    self.maybe_my_address = Some(bound_url.clone());

                    // respond to our parent
                    msg.respond(Ok(RequestToChildResponse::Bind(BindResultData {
                        bound_url: bound_url,
                    })));
                }
                RequestToChild::SendMessage { address, payload } => {
                    // make sure we have bound and get our address if so
                    //let my_addr = is_bound!(self, request_id, SendMessage);

                    // make sure we have bound and get our address if so
                    match &self.maybe_my_address {
                        None => {
                            msg.respond(Err(TransportError::new(
                                "Transport must be bound before sending".to_string(),
                            )));
                        }
                        Some(my_addr) => {
                            // get destinations server
                            let server_map = memory_server::MEMORY_SERVER_MAP.read().unwrap();
                            let maybe_server = server_map.get(&address);
                            if let None = maybe_server {
                                msg.respond(Err(TransportError::new(format!(
                                    "No Memory server at this address: {}",
                                    my_addr
                                ))));
                                continue;
                            }
                            let mut server = maybe_server.unwrap().lock().unwrap();

                            // if not already connected, request a connections
                            if self.connections.get(&address).is_none() {
                                match server.request_connect(&my_addr) {
                                    Err(err) => {
                                        msg.respond(Err(err));
                                        continue;
                                    }
                                    Ok(()) => self.connections.insert(address.clone()),
                                };
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

                            msg.respond(Ok(RequestToChildResponse::SendMessage));
                        }
                    };
                }
            }
        }

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
            return Err(format!("No Memory server at this address: {}", my_addr).into());
        }
        let mut server = maybe_server.unwrap().lock().unwrap();
        let (success, event_list) = server.process()?;
        if success {
            let mut to_connect_list: Vec<(Url)> = Vec::new();
            let mut non_connect_events = Vec::new();

            // process any connection events
            for event in event_list {
                match event {
                    TransportEvent::IncomingConnectionEstablished(in_cid) => {
                        let to_connect_uri =
                            Url::parse(&in_cid).expect("connectionId is not a valid Url");
                        to_connect_list.push(to_connect_uri.clone());
                        let mut channel_self = std::mem::replace(&mut self.channel_self, None);
                        channel_self.as_mut().expect("exists").publish(
                            RequestToParent::IncomingConnection {
                                address: to_connect_uri.clone(),
                            },
                        );
                        std::mem::replace(&mut self.channel_self, channel_self);
                    }
                    _ => non_connect_events.push(event),
                }
            }

            // Connect back to received connections if not already connected to them
            for remote_addr in to_connect_list {
                println!(
                    "(GhostTransportMemory)connecting {} <- {:?}",
                    remote_addr, my_addr
                );

                // if not already connected, request a connections
                if self.connections.get(&remote_addr).is_none() {
                    let _result = server.request_connect(&remote_addr);
                    self.connections.insert(remote_addr.clone());
                }
            }

            // process any other events
            for event in non_connect_events {
                match event {
                    TransportEvent::ReceivedData(from_addr, payload) => {
                        println!("RecivedData--- from:{:?} payload:{:?}", from_addr, payload);
                        let mut channel_self = std::mem::replace(&mut self.channel_self, None);
                        channel_self.as_mut().expect("exists").publish(
                            RequestToParent::ReceivedData {
                                address: Url::parse(&from_addr).unwrap(),
                                payload,
                            },
                        );
                        std::mem::replace(&mut self.channel_self, channel_self);
                    }
                    _ => panic!(format!("WHAT: {:?}", event)),
                };
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
    //use protocol::RequestToChildResponse;
    use lib3h_ghost_actor::GhostCallbackData;

    #[test]
    fn test_gmem_transport() {
        let mut transport1 = GhostTransportMemory::new();
        let mut t1_chan = transport1
            .take_parent_channel()
            .expect("exists")
            .as_context_channel::<Url>();
        let mut transport2 = GhostTransportMemory::new();
        let mut t2_chan = transport2
            .take_parent_channel()
            .expect("exists")
            .as_context_channel::<Url>();

        // create two memory bindings so that we have addresses
        assert_eq!(transport1.maybe_my_address, None);
        assert_eq!(transport2.maybe_my_address, None);

        let _expected_transport1_address = Url::parse("mem://addr_1").unwrap();
        t1_chan.request(
            std::time::Duration::from_millis(2000),
            Url::parse("mem://_").unwrap(),
            RequestToChild::Bind {
                spec: Url::parse("mem://_").unwrap(),
            },
            Box::new(|_, _, r| {
                println!("in transport1 bind callback, got: {:?}", r);
                match r {
                    GhostCallbackData::Response(Ok(RequestToChildResponse::Bind(BindResultData{bound_url}))) =>
                        assert_eq!(bound_url,Url::parse("mem://addr_1").unwrap()),
                    _ => assert!(false)
                }
                Ok(())
            }),
        );
        let expected_transport2_address = Url::parse("mem://addr_2").unwrap();
        t2_chan.request(
            std::time::Duration::from_millis(2000),
            Url::parse("mem://_").unwrap(),
            RequestToChild::Bind {
                spec: Url::parse("mem://_").unwrap(),
            },
            Box::new(|s, _, r| {
                let m = match s.downcast_mut::<GhostTransportMemory>() {
                    None => panic!("wrong type"),
                    Some(m) => m,
                };
                println!("in transport2 bind callback, got: {:?}", r);
                match r {
                    GhostCallbackData::Response(Ok(RequestToChildResponse::Bind(BindResultData{bound_url}))) =>
                        m.maybe_my_address = Some(bound_url.clone()),
                    _ => assert!(false)
                }
                Ok(())
            }),
        );

        transport1.process().unwrap();
        let _ = t1_chan.process(&mut transport1);

        transport2.process().unwrap();
        let _ = t2_chan.process(&mut transport2);

        assert_eq!(transport2.maybe_my_address,Some(expected_transport2_address));


        /*
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

                let requests = transport2.drain_requests();
                assert_eq!(
                    "[(None, IncomingConnection { address: \"mem://addr_1/\" }), (None, ReceivedData { address: \"mem://addr_1/\", payload: [116, 101, 115, 116, 32, 109, 101, 115, 115, 97, 103, 101] })]",
                    format!("{:?}", requests)
                );

                let mut r = transport1.drain_responses();
                let (_rid, response) = r.pop().unwrap();
                assert_eq!("SendMessage(Ok(()))", format!("{:?}", response));
        */
    }
}
