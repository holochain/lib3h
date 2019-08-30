#[macro_use]
extern crate detach;
#[macro_use]
extern crate lazy_static;

use detach::prelude::*;
use lib3h::transport::{error::*, protocol::*};
use lib3h_ghost_actor::prelude::*;
use std::{any::Any, collections::HashSet, sync::RwLock};
use url::Url;

enum ToParentContext {}
struct TestTransport {
    // instance name of this transport
    name: String,
    // our parent channel endpoint
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
                        bound_url: spec,
                    }))
                } else {
                    Err(TransportError::new("already bound".to_string()))
                };
                msg.respond(response);
            }
            RequestToChild::SendMessage {
                address: _,
                payload: _,
            } => panic!("BAM"),
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

// we need an "internet" that a transport can bind to that will
// deliver messages to bound transports
pub struct Mockernet {
    bindings: HashSet<Url>,
}
impl Mockernet {
    pub fn new() -> Self {
        Mockernet {
            bindings: HashSet::new(),
        }
    }
    pub fn bind(&mut self, url: Url) -> bool {
        if self.bindings.contains(&url) {
            return false;
        }
        self.bindings.insert(url)
    }
}

lazy_static! {
    pub static ref MOCKERNET: RwLock<Mockernet> = RwLock::new(Mockernet::new());
}

#[test]
fn ghost_transport() {
    let mut owner = TestTransportOwner::new();

    let mut t1: TransportActorParentWrapper<(), TestTransport> = GhostParentWrapper::new(
        TestTransport::new("t1"),
        "t1_requests", // prefix for request ids in the tracker
    );
    assert_eq!(t1.as_ref().name, "t1");
    /*    let t2: TransportActorParentWrapper<(),TestTransport> = GhostParentWrapper::new(
        TestTransport::new("t2"),
        "t2_requests", // prefix for request ids in the tracker
    );
    assert_eq!(t1.as_ref().name,"t2");
     */

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
    )
}
