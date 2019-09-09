#[macro_use]
extern crate detach;
#[macro_use]
extern crate lazy_static;

use detach::prelude::*;
use lib3h::transport::{error::*, protocol::*};
use lib3h_ghost_actor::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::RwLock,
};
use url::Url;

// We need an "internet" that a transport can bind to that will
// deliver messages to bound transports, we'll call it the Mockernet
pub struct Mockernet {
    bindings: HashMap<Url, Tube>,
    connections: HashMap<Url, HashSet<Url>>,
    errors: Vec<(Url, String)>,
}

// These are the events that the mockernet can generate that must by handled
// by any mockernet client.
pub enum MockernetEvent {
    Connection { from: Url },
    Message { from: Url, payload: Vec<u8> },
    Error(String),
}

// The Mockernet is a Series-of-Tubes, which is the technical term for the
// sets of crossbeam channels in the bindings that mockernet shuttles
// data between.
pub struct Tube {
    sender: crossbeam_channel::Sender<(Url, Vec<u8>)>,
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
            errors: Vec::new(),
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

    /// remove a binding, this is should trigger an error event
    pub fn unbind(&mut self, url: Url) {
        if self.bindings.contains_key(&url) {
            self.bindings.remove(&url);
            self.errors
                .push((url.clone(), format!("{} has become unbound", &url)));
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

        // push any errors for this url into the events
        let errors: Vec<_> = self.errors.drain(0..).collect();
        for (url, err) in &errors {
            if url == &address {
                events.push(MockernetEvent::Error(err.into()));
            } else {
                self.errors.push((url.clone(), err.clone()))
            }
        }

        let ref mut binding = match self.bindings.get(&address) {
            None => {
                if errors.len() == 0 {
                    return Err(format!("{} not bound", address));
                }
                return Ok(events);
            }
            Some(binding) => binding,
        };

        loop {
            match binding.receiver.try_recv() {
                Ok((from, payload)) => {
                    if !self.are_connected(&address, &from) {
                        events.push(MockernetEvent::Connection { from: from.clone() });
                    }
                    events.push(MockernetEvent::Message { from, payload });
                }
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(err) => return Err(format!("{:?}", err)),
            }
        }
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
    pub fn are_connected(&self, from: &Url, to: &Url) -> bool {
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
            TestTransport,
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

    fn take_parent_endpoint(&mut self) -> Option<TransportActorParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }
    // END BOILER PLATE--------------------------

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(self))?;
        for msg in self.endpoint_self.as_mut().drain_messages() {
            self.handle_RequestToChild(msg)?;
        }
        self.handle_events_from_mockernet()?;
        Ok(false.into())
    }
}

impl TestTransport {
    pub fn new(name: &str) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(
            endpoint_self
                .as_context_endpoint_builder()
                .request_id_prefix(&format!("{}_to_parent_", name))
                .build(),
        );
        TestTransport {
            name: name.to_string(),
            bound_url: None,
            endpoint_parent,
            endpoint_self,
        }
    }

    /// private dispatcher for messages coming from our parent
    fn handle_RequestToChild(
        &mut self,
        mut msg: ToChildMessage,
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
                msg.respond(response)?;
            }
            RequestToChild::SendMessage { uri, payload } => {
                if self.bound_url.is_none() {
                    msg.respond(Err(TransportError::new(format!("{} not bound", self.name))))?;
                } else {
                    let mut mockernet = MOCKERNET.write().unwrap();
                    // return error if not bound.
                    let response = match mockernet.send_to(
                        address,
                        self.bound_url.as_ref().unwrap().clone(),
                        payload,
                    ) {
                        Err(err) => Err(TransportError::new(err)),
                        Ok(()) => Ok(RequestToChildResponse::SendMessage),
                    };
                    msg.respond(response)?;
                }
            }
        }
        Ok(())
    }

    fn handle_events_from_mockernet(&mut self) -> GhostResult<()> {
        let mut mockernet = MOCKERNET.write().unwrap();
        let our_url = self.bound_url.as_ref().unwrap();
        if let Ok(events) = mockernet.process_for(our_url.clone()) {
            for e in events {
                match e {
                    MockernetEvent::Message { from, payload } => {
                        self.endpoint_self.publish(RequestToParent::ReceivedData {
                            address: from,
                            payload,
                        })?;
                    }
                    MockernetEvent::Connection { from } => {
                        self.endpoint_self
                            .publish(RequestToParent::IncomingConnection { uri: from })?;
                    }
                    MockernetEvent::Error(err) => {
                        self.endpoint_self
                            .publish(RequestToParent::ErrorOccured {
                                uri: our_url.clone(),
                                error: TransportError::new(err),
                            })?;
                    }
                }
            }
        }
        Ok(())
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
    // create an object that can be used to hold state data in callbacks to the transports
    let mut owner = TestTransportOwner::new();

    let mut t1: TransportActorParentWrapper<TestTransportOwner, (), TestTransport> =
        GhostParentWrapper::new(
            TestTransport::new("t1"),
            "t1_requests", // prefix for request ids in the tracker
        );
    assert_eq!(t1.as_ref().name, "t1");
    let mut t2: TransportActorParentWrapper<TestTransportOwner, (), TestTransport> =
        GhostParentWrapper::new(
            TestTransport::new("t2"),
            "t2_requests", // prefix for request ids in the tracker
        );
    assert_eq!(t2.as_ref().name, "t2");

    // bind t1 to the network
    t1.request(
        (),
        RequestToChild::Bind {
            spec: Url::parse("mocknet://t1").expect("can parse url"),
        },
        // callback should simply log the response
        Box::new(|owner, _, response| {
            owner.log.push(format!("{:?}", response));
            Ok(())
        }),
    )
    .unwrap();
    t1.process(&mut owner).expect("should process");
    assert_eq!(
        "\"Response(Ok(Bind(BindResultData { bound_url: \\\"mocknet://t1/\\\" })))\"",
        format!("{:?}", owner.log[0])
    );

    // lets do some things to test out returning of error messages, i.e. sending messages
    // to someone not bount to the network
    t1.request(
        (),
        RequestToChild::SendMessage {
            uri: Url::parse("mocknet://t2").expect("can parse url"),
            payload: b"won't be received!".to_vec(),
        },
        // callback should simply log the response
        Box::new(|owner, _, response| {
            owner.log.push(format!("{:?}", response));
            Ok(())
        }),
    )
    .unwrap();

    t1.process(&mut owner).expect("should process");
    assert_eq!(
        "\"Response(Err(TransportError(\\\"mocknet://t2/ not bound\\\")))\"",
        format!("{:?}", owner.log[1])
    );

    // bind t2 to the network
    t2.request(
        (),
        RequestToChild::Bind {
            spec: Url::parse("mocknet://t2").expect("can parse url"),
        },
        // callback should simply log the response
        Box::new(|owner, _, response| {
            owner.log.push(format!("{:?}", response));
            Ok(())
        }),
    )
    .unwrap();
    t2.process(&mut owner).expect("should process");
    assert_eq!(
        "\"Response(Ok(Bind(BindResultData { bound_url: \\\"mocknet://t2/\\\" })))\"",
        format!("{:?}", owner.log[2])
    );

    t1.request(
        (),
        RequestToChild::SendMessage {
            uri: Url::parse("mocknet://t2").expect("can parse url"),
            payload: b"foo".to_vec(),
        },
        // callback should simply log the response
        Box::new(|owner, _, response| {
            owner.log.push(format!("{:?}", response));
            Ok(())
        }),
    )
    .unwrap();

    // we should get back an Ok on having sent the message when t1 gets processed
    t1.process(&mut owner).expect("should process");
    assert_eq!(
        "\"Response(Ok(SendMessage))\"",
        format!("{:?}", owner.log[3])
    );

    // and when we drain messages from t2 we should see both the incoming connection from t1
    // and the message itself
    t2.process(&mut owner).expect("should process");
    let mut messages = t2.drain_messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(
        "IncomingConnection { uri: \"mocknet://t1/\" }",
        format!("{:?}", messages[0].take_message().expect("exists"))
    );
    assert_eq!(
        "ReceivedData { address: \"mocknet://t1/\", payload: [102, 111, 111] }",
        format!("{:?}", messages[1].take_message().expect("exists"))
    );

    {
        let mut mockernet = MOCKERNET.write().unwrap();
        mockernet.unbind(Url::parse("mocknet://t1").expect("can parse url"));
    }
    t1.process(&mut owner).expect("should process");
    let mut messages = t1.drain_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(
        "TransportError { error: TransportError(\"mocknet://t1/ has become unbound\") }",
        format!("{:?}", messages[0].take_message().expect("exists"))
    );
}
