use crate::transport::error::TransportError;
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::data_types::Opaque;
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
