//! This is a stub of the keystore so we can prove out the encoding transport

use crate::{
    error::{Lib3hError, Lib3hResult},
    transport::TransportEncoding,
};

use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
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

use keystore_protocol::*;

pub type DynKeystoreActor = Box<
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

pub type KeystoreActorParentWrapperDyn<Context> = GhostParentWrapperDyn<
    TransportEncoding,
    Context,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    Lib3hError,
>;

type KeystoreParentEndpoint = GhostEndpoint<
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    Lib3hError,
>;

type KeystoreSelfEndpoint = GhostContextEndpoint<
    KeystoreStub,
    (),
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    Lib3hError,
>;

type KeystoreMessageFromParent =
    GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, Lib3hError>;

pub struct KeystoreStub {
    endpoint_parent: Option<KeystoreParentEndpoint>,
    endpoint_self: Detach<KeystoreSelfEndpoint>,
}

impl KeystoreStub {
    pub fn new() -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(
            endpoint_self
                .as_context_endpoint_builder()
                .request_id_prefix("keystore_to_parent_")
                .build(),
        );
        Self {
            endpoint_parent,
            endpoint_self,
        }
    }

    fn handle_msg_from_parent(&mut self, mut msg: KeystoreMessageFromParent) -> Lib3hResult<()> {
        match msg.take_message().expect("exists") {
            RequestToChild::Sign { id, payload } => self.handle_sign(msg, id, payload),
        }
    }

    fn handle_sign(
        &mut self,
        msg: KeystoreMessageFromParent,
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
    fn take_parent_endpoint(&mut self) -> Option<KeystoreParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(self))?;
        for msg in self.endpoint_self.as_mut().drain_messages() {
            self.handle_msg_from_parent(msg).expect("no ghost errors");
        }
        Ok(false.into())
    }
}
