use crate::{dht::dht_protocol::*, error::*, transport};
use lib3h_ghost_actor::prelude::*;
use lib3h_protocol::data_types::*;

/// Gateway protocol enums for use with GhostActor implementation
#[derive(Debug)]
pub enum GatewayRequestToChild {
    Transport(transport::protocol::RequestToChild),
    Dht(DhtRequestToChild),
    Bootstrap(BootstrapData),
    SendAll(Vec<u8>),
}

#[derive(Debug)]
pub enum GatewayRequestToChildResponse {
    Transport(transport::protocol::RequestToChildResponse),
    Dht(DhtRequestToChildResponse),
    BootstrapSuccess,
}

#[derive(Debug)]
pub enum GatewayRequestToParent {
    Transport(transport::protocol::RequestToParent),
    Dht(DhtRequestToParent),
}

#[derive(Debug)]
pub enum GatewayRequestToParentResponse {
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
pub type GatewaySelfEndpoint<UserData> = GhostContextEndpoint<
    UserData,
    GatewayRequestToParent,
    GatewayRequestToParentResponse,
    GatewayRequestToChild,
    GatewayRequestToChildResponse,
    Lib3hError,
>;
pub type GatewayParentWrapper<UserData, Actor> = GhostParentWrapper<
    UserData,
    GatewayRequestToParent,
    GatewayRequestToParentResponse,
    GatewayRequestToChild,
    GatewayRequestToChildResponse,
    Lib3hError,
    Actor,
>;
pub type GatewayParentWrapperDyn<UserData> = GhostParentWrapperDyn<
    UserData,
    GatewayRequestToParent,
    GatewayRequestToParentResponse,
    GatewayRequestToChild,
    GatewayRequestToChildResponse,
    Lib3hError,
>;
