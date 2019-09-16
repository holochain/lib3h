use crate::transport::error::TransportError;
use detach::prelude::*;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::data_types::Opaque;
use lib3h_tracing::Lib3hSpan;
use url::Url;

#[derive(Debug, Clone)]
pub struct BindResultData {
    pub bound_url: Url,
}

/// Transport protocol enums for use with GhostActor implementation
#[derive(Debug, Clone)]
pub enum RequestToChild {
    Bind { spec: Url }, // wss://0.0.0.0:0 -> all network interfaces first available port
    SendMessage { uri: Url, payload: Opaque },
}

#[derive(Debug, Clone)]
pub enum RequestToChildResponse {
    Bind(BindResultData),
    SendMessageSuccess,
}

#[derive(Debug, Clone)]
pub enum RequestToParent {
    // TODO remove `uri` field once we have refactored how we handle Connection/Disconnection
    ErrorOccured { uri: Url, error: TransportError },
    IncomingConnection { uri: Url },
    ReceivedData { uri: Url, payload: Opaque },
}

#[derive(Debug, Clone)]
pub enum RequestToParentResponse {
    // N/A
}

pub type ToChildMessage =
    GhostMessage<RequestToChild, RequestToParent, RequestToChildResponse, TransportError>;

pub type ToParentMessage =
    GhostMessage<RequestToParent, RequestToChild, RequestToParentResponse, TransportError>;

pub type DynTransportActor = Box<
    dyn GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        TransportError,
    >,
>;

/// this is awkward... I *think* it'll go away with ghost-actor proc-macro
pub struct TransportEndpointAsActor {
    endpoint_parent: Option<TransportActorParentEndpoint>,
    endpoint_self: Detach<
        GhostContextEndpoint<
            Self,
            RequestToParent,
            RequestToParentResponse,
            RequestToChild,
            RequestToChildResponse,
            TransportError,
        >,
    >,
    endpoint_inner: Detach<TransportActorParentContextEndpoint<Self>>,
}

impl TransportEndpointAsActor {
    pub fn new(inner: TransportActorParentEndpoint) -> Self {
        let (endpoint_parent, endpoint_self) = create_ghost_channel();
        let endpoint_parent = Some(endpoint_parent);
        let endpoint_self = Detach::new(
            endpoint_self
                .as_context_endpoint_builder()
                .request_id_prefix("enc_to_parent_")
                .build(),
        );
        Self {
            endpoint_parent,
            endpoint_self,
            endpoint_inner: Detach::new(inner.as_context_endpoint_builder().build::<Self>()),
        }
    }
}

impl
    GhostActor<
        RequestToParent,
        RequestToParentResponse,
        RequestToChild,
        RequestToChildResponse,
        TransportError,
    > for TransportEndpointAsActor
{
    fn take_parent_endpoint(&mut self) -> Option<TransportActorParentEndpoint> {
        std::mem::replace(&mut self.endpoint_parent, None)
    }

    fn process_concrete(&mut self) -> GhostResult<WorkWasDone> {
        detach_run!(&mut self.endpoint_self, |es| es.process(self))?;
        for mut msg in self.endpoint_self.as_mut().drain_messages() {
            let data = msg.take_message().expect("exists");
            self.endpoint_inner.request(
                Lib3hSpan::todo(),
                data,
                Box::new(move |_, response| {
                    msg.respond(match response {
                        GhostCallbackData::Timeout => Err("timeout".into()),
                        GhostCallbackData::Response(r) => r,
                    })?;
                    Ok(())
                }),
            )?;
        }
        detach_run!(&mut self.endpoint_inner, |it| it.process(self))?;
        for mut msg in self.endpoint_inner.as_mut().drain_messages() {
            let data = msg.take_message().expect("exists");
            self.endpoint_self.publish(Lib3hSpan::todo(), data)?;
        }
        Ok(false.into())
    }
}

pub type TransportActorParentEndpoint = GhostEndpoint<
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    TransportError,
>;

pub type TransportActorParentContextEndpoint<UserData> = GhostContextEndpoint<
    UserData,
    RequestToChild,
    RequestToChildResponse,
    RequestToParent,
    RequestToParentResponse,
    TransportError,
>;

pub type TransportActorSelfEndpoint<UserData> = GhostContextEndpoint<
    UserData,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    TransportError,
>;
pub type TransportActorParentWrapper<UserData, Actor> = GhostParentWrapper<
    UserData,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    TransportError,
    Actor,
>;
pub type TransportActorParentWrapperDyn<UserData> = GhostParentWrapperDyn<
    UserData,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    TransportError,
>;
