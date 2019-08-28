//! This is a stub of the keystore so we can prove out the encoding transport

use crate::error::{Lib3hError, Lib3hResult};
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use std::any::Any;

pub mod keystore_protocol {
    #[derive(Debug)]
    pub enum RequestToChild {
        Sign { id: String, payload: Vec<u8> },
    }

    #[derive(Debug)]
    pub enum RequestToChildResponse {
        Sign { signature: Vec<u8> },
    }

    #[derive(Debug)]
    pub enum RequestToParent {}

    #[derive(Debug)]
    pub enum RequestToParentResponse {}
}

pub type KeystoreActor = Box<
    dyn GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Lib3hError,
    >,
>;
pub type KeystoreActorParentEndpoint = GhostEndpoint<
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    Lib3hError,
>;
pub type KeystoreActorParentWrapper<Context> = GhostParentWrapper<
    Context,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    Lib3hError,
>;

use keystore_protocol::*;

pub struct KeystoreStub {
    endpoint_parent: Option<
        GhostEndpoint<
            RequestToChild,
            RequestToChildResponse,
            RequestToParent,
            RequestToParentResponse,
            Lib3hError,
        >,
    >,
    endpoint_self: Detach<
        GhostContextEndpoint<
            (),
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            Lib3hError,
        >,
    >,
}

impl KeystoreStub {
    pub fn new() -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(endpoint_self.as_context_endpoint("keystore_to_parent_"));
        Self {
            endpoint_parent,
            endpoint_self,
        }
    }

    fn handle_msg_from_parent(
        &mut self,
        mut msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, Lib3hError>,
    ) -> Lib3hResult<()> {
        match msg.take_message().expect("exists") {
            RequestToChild::Sign { id, payload } => self.handle_sign(msg, id, payload),
        }
    }

    fn handle_sign(
        &mut self,
        msg: GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, Lib3hError>,
        _id: String,
        _payload: Vec<u8>,
    ) -> Lib3hResult<()> {
        // THIS IS A STUB, just responding with empty signature
        msg.respond(Ok(RequestToChildResponse::Sign {
            signature: b"".to_vec(),
        }));
        Ok(())
    }
}

impl
    GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        Lib3hError,
    > for KeystoreStub
{
    fn as_any(&mut self) -> &mut dyn Any {
        &mut *self
    }

    fn take_parent_endpoint(
        &mut self,
    ) -> Option<
        GhostEndpoint<
            RequestToChild,
            RequestToChildResponse,
            RequestToParent,
            RequestToParentResponse,
            Lib3hError,
        >,
    > {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(self.as_any()))?;
        for msg in self.endpoint_self.as_mut().drain_messages() {
            self.handle_msg_from_parent(msg).expect("no ghost errors");
        }
        Ok(false.into())
    }
}
