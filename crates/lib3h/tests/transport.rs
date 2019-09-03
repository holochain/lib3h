#[macro_use]
extern crate detach;
#[macro_use]
extern crate lazy_static;

use detach::prelude::*;
use lib3h::transport::{error::*, protocol::*};
use lib3h_ghost_actor::prelude::*;
use std::{
    any::Any,
    collections::{HashMap, HashSet},
    sync::RwLock,
};
use url::Url;

// we need an "internet" that a transport can bind to that will
// deliver messages to bound transports, we'll call it the Mockernet
pub struct Mockernet {
    bindings: HashMap<Url, Tube>,
    connections: HashMap<Url, HashSet<Url>>,
}

pub enum MockernetEvent {
    Connection { from: Url },
    Message { from: Url, payload: Vec<u8> },
    //    Error(String),
}

// The Mockernet is a Series-of-Tubes, which is the technical term for the
// sets of crossbeam channels in the bindings that mockernet shuttles
// data between.
// The Mockernet simulates a
pub struct Tube {
    sender: crossbeam_channel::Sender<(Url, Vec<u8>)>,
    #[allow(dead_code)]
    receiver: crossbeam_channel::Receiver<(Url, Vec<u8>)>,
}
impl Tube {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded::<(Url, Vec<u8>)>();
        Tube { sender, receiver }
    }
}
impl Mockernet {
    pub fn new() -> Self {
        Mockernet {
            bindings: HashMap::new(),
            connections: HashMap::new(),
        }
    }

    /// create a binding to the mockernet, without one you can't send or receive
    pub fn bind(&mut self, url: Url) -> bool {
        if self.bindings.contains_key(&url) {
            false
        } else {
            self.bindings.insert(url, Tube::new());
            true
        }
    }

    /// send a message to anyone on the Mockernet
    pub fn send_to(&mut self, to: Url, from: Url, payload: Vec<u8>) -> Result<(), String> {
        {
            let _src = self
                .bindings
                .get(&from)
                .ok_or(format!("{} not bound", from))?;
            let dst = self.bindings.get(&to).ok_or(format!("{} not bound", to))?;
            dst.sender
                .send((from.clone(), payload))
                .map_err(|e| format!("{}", e))?;
        }
        self.connect(from.clone(), to.clone());
        Ok(())
    }

    /// check to see, for a given Url, if there are any events waiting
    pub fn process_for(&mut self, address: Url) -> Result<Vec<MockernetEvent>, String> {
        let mut events = Vec::new();
        let binding = self
            .bindings
            .get(&address)
            .ok_or(format!("{} not bound", address))?;
        let (from, payload) = binding
            .receiver
            .try_recv()
            .map_err(|e| format!("{:?}", e))?;
        if !self.are_connected(&address, &from) {
            events.push(MockernetEvent::Connection { from: from.clone() });
        }
        events.push(MockernetEvent::Message { from, payload });
        Ok(events)
    }

    /// record a connection
    pub fn connect(&mut self, from: Url, to: Url) {
        if let Some(cmap) = self.connections.get_mut(&from) {
            cmap.insert(to);
        } else {
            let mut cmap = HashSet::new();
            cmap.insert(to);
            self.connections.insert(from, cmap);
        }
    }

    /// check to see if two nodes are connected
    pub fn are_connected(&mut self, from: &Url, to: &Url) -> bool {
        match self.connections.get(from) {
            None => return false,
            Some(cmap) => cmap.contains(to),
        }
    }
}

lazy_static! {
    pub static ref MOCKERNET: RwLock<Mockernet> = RwLock::new(Mockernet::new());
}

enum ToParentContext {}
struct TestTransport {
    // instance name of this transport
    name: String,
    // our parent channel endpoint
    bound_url: Option<Url>,
    endpoint_parent: Option<TransportActorParentEndpoint>,
    // our self channel endpoint
    endpoint_self: Detach<
        GhostContextEndpoint<
            ToParentContext,
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            TransportError,
        >,
    >,
}

impl
    GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        TransportError,
    > for TestTransport
{
    // START BOILER PLATE--------------------------
    fn as_any(&mut self) -> &mut dyn Any {
        &mut *self
    }

    fn take_parent_endpoint(&mut self) -> Option<TransportActorParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(self.as_any()))?;
        for msg in self.endpoint_self.as_mut().drain_messages() {
            self.handle_msg_from_parent(msg)?;
        }
        self.handle_events_from_mockernet();
        Ok(false.into())
    }
    // END BOILER PLATE--------------------------
}

impl TestTransport {
    pub fn new(name: &str) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self =
            Detach::new(endpoint_self.as_context_endpoint(&format!("{}_to_parent_", name)));
        TestTransport {
            name: name.to_string(),
            bound_url: None,
            endpoint_parent,
            endpoint_self,
        }
    }

    /// private dispatcher for messages coming from our parent
    fn handle_msg_from_parent(
        &mut self,
        mut msg: GhostMessage<
            RequestToChild,
            RequestToParent,
            RequestToChildResponse,
            TransportError,
        >,
    ) -> TransportResult<()> {
        match msg.take_message().expect("exists") {
            RequestToChild::Bind { spec } => {
                let mut mockernet = MOCKERNET.write().unwrap();
                let response = if mockernet.bind(spec.clone()) {
                    Ok(RequestToChildResponse::Bind(BindResultData {
                        bound_url: spec.clone(),
                    }))
                } else {
                    Err(TransportError::new("already bound".to_string()))
                };
                self.bound_url = Some(spec);
                msg.respond(response);
            }
            RequestToChild::SendMessage { address, payload } => {
                let mut mockernet = MOCKERNET.write().unwrap();
                // return error if not bound.
                let _ =
                    mockernet.send_to(address, self.bound_url.as_ref().unwrap().clone(), payload);
                //                msg.respond(response);
            }
        }
        Ok(())
    }

    fn handle_events_from_mockernet(&mut self) {
        let mut mockernet = MOCKERNET.write().unwrap();
        let our_url = self.bound_url.as_ref().unwrap();
        if let Ok(events) = mockernet.process_for(our_url.clone()) {
            for e in events {
                match e {
                    MockernetEvent::Message { from, payload } => {
                        detach_run!(self.endpoint_self, |s| s.publish(
                            RequestToParent::ReceivedData {
                                address: from,
                                payload
                            }
                        ));
                    }
                    MockernetEvent::Connection { from } => {
                        detach_run!(self.endpoint_self, |s| s
                            .publish(RequestToParent::IncomingConnection { address: from }));
                    }
                }
            }
        }
    }
}

// owner object for the transport tests with a log into which
// results can go for testing purposes
struct TestTransportOwner {
    log: Vec<String>,
}
impl TestTransportOwner {
    fn new() -> Self {
        TestTransportOwner { log: Vec::new() }
    }
}

#[test]
fn ghost_transport() {
    let mut owner = TestTransportOwner::new();

    let mut t1: TransportActorParentWrapper<(), TestTransport> = GhostParentWrapper::new(
        TestTransport::new("t1"),
        "t1_requests", // prefix for request ids in the tracker
    );
    assert_eq!(t1.as_ref().name, "t1");
    let mut t2: TransportActorParentWrapper<(), TestTransport> = GhostParentWrapper::new(
        TestTransport::new("t2"),
        "t2_requests", // prefix for request ids in the tracker
    );
    assert_eq!(t2.as_ref().name, "t2");

    // bind t1 to the network
    t1.request(
        std::time::Duration::from_millis(2000),
        (),
        RequestToChild::Bind {
            spec: Url::parse("mocknet://t1").expect("can parse url"),
        },
        // callback should simply log the response
        Box::new(|dyn_owner, _, response| {
            let owner = dyn_owner
                .downcast_mut::<TestTransportOwner>()
                .expect("a TestTransportOwner");
            owner.log.push(format!("{:?}", response));
            Ok(())
        }),
    );
    t1.process(&mut owner).expect("should process");
    assert_eq!(
        "\"Response(Ok(Bind(BindResultData { bound_url: \\\"mocknet://t1/\\\" })))\"",
        format!("{:?}", owner.log[0])
    );

    // bind t2 to the network
    t2.request(
        std::time::Duration::from_millis(2000),
        (),
        RequestToChild::Bind {
            spec: Url::parse("mocknet://t2").expect("can parse url"),
        },
        // callback should simply log the response
        Box::new(|dyn_owner, _, response| {
            let owner = dyn_owner
                .downcast_mut::<TestTransportOwner>()
                .expect("a TestTransportOwner");
            owner.log.push(format!("{:?}", response));
            Ok(())
        }),
    );
    t2.process(&mut owner).expect("should process");
    assert_eq!(
        "\"Response(Ok(Bind(BindResultData { bound_url: \\\"mocknet://t2/\\\" })))\"",
        format!("{:?}", owner.log[1])
    );

    t1.request(
        std::time::Duration::from_millis(2000),
        (),
        RequestToChild::SendMessage {
            address: Url::parse("mocknet://t2").expect("can parse url"),
            payload: b"foo".to_vec(),
        },
        // callback should simply log the response
        Box::new(|dyn_owner, _, response| {
            let owner = dyn_owner
                .downcast_mut::<TestTransportOwner>()
                .expect("a TestTransportOwner");
            owner.log.push(format!("{:?}", response));
            Ok(())
        }),
    );
    t1.process(&mut owner).expect("should process");
    t2.process(&mut owner).expect("should process");
    let mut messages = t2.drain_messages();
    assert_eq!(messages.len(), 2);

    // because this is the first incoming message, the low level
    // transport should also send an IncomingConnection event
    assert_eq!(
        "IncomingConnection { address: \"mocknet://t1/\" }",
        format!("{:?}", messages[0].take_message().expect("exists"))
    );
    assert_eq!(
        "ReceivedData { address: \"mocknet://t1/\", payload: [102, 111, 111] }",
        format!("{:?}", messages[1].take_message().expect("exists"))
    );
}
