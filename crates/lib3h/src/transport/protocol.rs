use crate::transport::{error::TransportError, ConnectionId};
use url::Url;
use lib3h_ghost_actor::prelude::*;

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
// Transport protocol for GhostChannel
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

pub type TransportParentEndpointWithContext = GhostParentContextEndpoint<
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
    Bind {
        parent_msg: GhostMessage<
            TransportRequestToChild,
            TransportRequestToParent,
            TransportRequestToChildResponse,
            TransportError>,
    },
    SendMessage {
        parent_msg: GhostMessage<
            TransportRequestToChild,
            TransportRequestToParent,
            TransportRequestToChildResponse,
            TransportError>,
    },
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