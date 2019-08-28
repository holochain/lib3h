use crate::transport::{error::TransportError, ConnectionId};
use lib3h_ghost_actor::prelude::*;
use url::Url;

/// Commands that can be sent to an implementor of the Transport trait and handled during `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum TransportCommand {
    Connect(Url, /*request_id*/ String),
    Send(Vec<ConnectionId>, Vec<u8>),
    SendAll(Vec<u8>),
    Close(ConnectionId),
    CloseAll,
    Bind(Url),
}

/// Events that can be generated during a `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum TransportEvent {
    /// Notify that some TransportError occured
    ErrorOccured(ConnectionId, TransportError),
    /// an outgoing connection has been established
    ConnectResult(ConnectionId, /*request_id*/ String),
    /// we have received an incoming connection
    IncomingConnectionEstablished(ConnectionId),
    /// We have received data from a connection
    ReceivedData(ConnectionId, Vec<u8>),
    /// A connection closed for whatever reason
    ConnectionClosed(ConnectionId),
}

//--------------------------------------------------------------------------------------------------
// Transport protocol for GhostActors
//--------------------------------------------------------------------------------------------------

pub type TransportEndpoint = GhostEndpoint<
    TransportRequestToChild,
    TransportRequestToChildResponse,
    TransportRequestToParent,
    TransportRequestToParentResponse,
    TransportError,
>;

pub type TransportEndpointWithContext = GhostContextEndpoint<
    TransportContext,
    TransportRequestToParent,
    TransportRequestToParentResponse,
    TransportRequestToChild,
    TransportRequestToChildResponse,
    TransportError,
>;

pub type TransportParentWrapper = GhostParentWrapper<
    TransportContext,
    TransportRequestToParent,
    TransportRequestToParentResponse,
    TransportRequestToChild,
    TransportRequestToChildResponse,
    TransportError,
>;

pub type TransportMessage = GhostMessage<
    TransportRequestToParent,
    TransportRequestToChild,
    TransportRequestToParentResponse,
    TransportError,
>;

#[derive(Debug)]
enum TransportContext {
    Bind { parent_msg: TransportMessage },
    SendMessage { parent_msg: TransportMessage },
}

/// Transport protocol enums for use with GhostActor implementation
#[derive(Debug)]
pub enum TransportRequestToChild {
    Bind { spec: Url }, // wss://0.0.0.0:0 -> all network interfaces first available port
    SendMessage { address: Url, payload: Vec<u8> },
}

#[derive(Debug)]
pub enum TransportRequestToChildResponse {
    Bind(BindResultData),
    SendMessage,
}

#[derive(Debug)]
pub enum TransportRequestToParent {
    IncomingConnection { address: Url },
    ReceivedData { address: Url, payload: Vec<u8> },
    TransportError { error: TransportError },
}

#[derive(Debug)]
pub enum TransportRequestToParentResponse {
    Allowed,    // just for testing
    Disallowed, // just for testing
}

#[derive(Debug)]
pub struct BindResultData {
    pub bound_url: Url,
}
