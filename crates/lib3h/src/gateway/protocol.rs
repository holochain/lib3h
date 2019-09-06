use lib3h_ghost_actor::prelude::*;
use url::Url;
use crate::dht::dht_protocol::*;

/// Gateway protocol enums for use with GhostActor implementation
#[derive(Debug)]
pub enum GatewayRequestToChild {
    // FIXME
    Transport(transport::protocol::RequestToChild),
    Dht(DhtRequestToChild),
}

#[derive(Debug)]
pub enum GatewayRequestToChildResponse {
    // FIXME
    Transport(transport::protocol::RequestToChildResponse),
    Dht(DhtRequestToChildResponse),
}

#[derive(Debug)]
pub enum GatewayRequestToParent {
    // FIXME
    Transport(transport::protocol::RequestToParent),
    Dht(DhtRequestToParent),
}

#[derive(Debug)]
pub enum GatewayRequestToParentResponse {
    // FIXME
    Transport(transport::protocol::RequestToParentResponse),
    Dht(DhtRequestToParentResponse),
}

pub type GatewayToChildMessage =
GhostMessage<GatewayRequestToChild, GatewayRequestToParent, GatewayRequestToChildResponse, Lib3hError>;

pub type GatewayToParentMessage =
GhostMessage<GatewayRequestToParent, GatewayRequestToChild, GatewayRequestToParentResponse, Lib3hError>;

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
