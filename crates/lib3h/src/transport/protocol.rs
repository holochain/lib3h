use crate::transport::error::TransportError;
use lib3h_ghost_actor::prelude::*;
use url::Url;

#[derive(Debug, Clone)]
pub struct BindResultData {
    pub bound_url: Url,
}

/// Transport protocol enums for use with GhostActor implementation
#[derive(Debug, Clone)]
pub enum RequestToChild {
    Bind { spec: Url }, // wss://0.0.0.0:0 -> all network interfaces first available port
    SendMessage { uri: Url, payload: Vec<u8> },
}

#[derive(Debug, Clone)]
pub enum RequestToChildResponse {
    Bind(BindResultData),
    SendMessage { payload: Vec<u8> },
}

#[derive(Debug, Clone)]
pub enum RequestToParent {
    ErrorOccured { uri: Url, error: TransportError },
    IncomingConnection { uri: Url },
    ReceivedData { uri: Url, payload: Vec<u8> },
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
pub type TransportEndpointWithContext<UserData, Context> =
    TransportActorSelfEndpoint<UserData, Context>;
pub type TransportActorSelfEndpoint<UserData, Context> = GhostContextEndpoint<
    UserData,
    Context,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    TransportError,
>;
pub type TransportActorParentWrapper<UserData, Context, Actor> = GhostParentWrapper<
    UserData,
    Context,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    TransportError,
    Actor,
>;
pub type ChildTransportWrapperDyn<UserData, Context> =
    TransportActorParentWrapperDyn<UserData, Context>;
pub type TransportActorParentWrapperDyn<UserData, Context> = GhostParentWrapperDyn<
    UserData,
    Context,
    RequestToParent,
    RequestToParentResponse,
    RequestToChild,
    RequestToChildResponse,
    TransportError,
>;
