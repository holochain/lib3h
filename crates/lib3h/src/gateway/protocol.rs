use crate::{
    dht::dht_protocol::*,
    error::*,
    transport,
};
use lib3h_ghost_actor::prelude::*;
use url::Url;

#[derive(Debug, Clone)]
pub enum GatewayContext {
    Transport {parent_request: transport::protocol::ToChildMessage},
    Dht {parent_request: DhtToChildMessage},
    Bind {
        maybe_parent_msg: Option<transport::protocol::ToChildMessage>,
    },
    SendMessage {
        maybe_parent_msg: Option<transport::protocol::ToChildMessage>,
    },
}

/// Gateway protocol enums for use with GhostActor implementation
#[derive(Debug, Clone)]
pub enum GatewayRequestToChild {
    // FIXME
    Transport(transport::protocol::RequestToChild),
    Dht(DhtRequestToChild),
}

#[derive(Debug, Clone)]
pub enum GatewayRequestToChildResponse {
    // FIXME
    Transport(transport::protocol::RequestToChildResponse),
    Dht(DhtRequestToChildResponse),
}

#[derive(Debug, Clone)]
pub enum GatewayRequestToParent {
    // FIXME
    Transport(transport::protocol::RequestToParent),
    Dht(DhtRequestToParent),
}

#[derive(Debug, Clone)]
pub enum GatewayRequestToParentResponse {
    // FIXME
    Transport(transport::protocol::RequestToParentResponse),
    Dht(DhtRequestToParentResponse),
}

pub type GatewayToChildMessage = GhostMessage<
    GatewayRequestToChild,
    GatewayRequestToParent,
    GatewayRequestToChildResponse,
    Lib3hError,
>;

pub type GatewayToParentMessage = GhostMessage<
    GatewayRequestToParent,
    GatewayRequestToChild,
    GatewayRequestToParentResponse,
    Lib3hError,
>;

pub type GatewayActor = dyn GhostActor<
    GatewayRequestToParent,
    GatewayRequestToParentResponse,
    GatewayRequestToChild,
    GatewayRequestToChildResponse,
    Lib3hError,
>;

pub type DynGatewayActor = Box<
    dyn GhostActor<
        GatewayRequestToParent,
        GatewayRequestToParentResponse,
        GatewayRequestToChild,
        GatewayRequestToChildResponse,
        Lib3hError,
    >,
>;

pub type GatewayParentEndpoint = GhostEndpoint<
    GatewayRequestToChild,
    GatewayRequestToChildResponse,
    GatewayRequestToParent,
    GatewayRequestToParentResponse,
    Lib3hError,
>;
pub type GatewaySelfEndpoint<UserData, Context> = GhostContextEndpoint<
    UserData,
    Context,
    GatewayRequestToParent,
    GatewayRequestToParentResponse,
    GatewayRequestToChild,
    GatewayRequestToChildResponse,
    Lib3hError,
>;
pub type GatewayParentWrapper<UserData, Context, Actor> = GhostParentWrapper<
    UserData,
    Context,
    GatewayRequestToParent,
    GatewayRequestToParentResponse,
    GatewayRequestToChild,
    GatewayRequestToChildResponse,
    Lib3hError,
    Actor,
>;
pub type GatewayParentWrapperDyn<UserData, Context> = GhostParentWrapperDyn<
    UserData,
    Context,
    GatewayRequestToParent,
    GatewayRequestToParentResponse,
    GatewayRequestToChild,
    GatewayRequestToChildResponse,
    Lib3hError,
>;
