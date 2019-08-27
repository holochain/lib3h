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

pub type TransportChannel = GhostChannel<
    TransportRequestToChild<Url>,
    TransportRequestToChildResponse<Url>,
    TransportRequestToParent<Url>,
    TransportRequestToParentResponse,
    TransportError,
>;

pub type TransportChannelWithContext = GhostContextChannel<
    TransportContext,
    TransportRequestToParent<Url>,
    TransportRequestToParentResponse,
    TransportRequestToChild<Url>,
    TransportRequestToChildResponse<Url>,
    TransportError,
>;

pub type TransportParentChannelWithContext = GhostParentContextChannel<
    TransportContext,
    TransportRequestToParent<Url>,
    TransportRequestToParentResponse,
    TransportRequestToChild<Url>,
    TransportRequestToChildResponse<Url>,
    TransportError,
>;

pub type TransportMessage = GhostMessage<
    TransportRequestToParent<Url>,
    TransportRequestToChild<Url>,
    TransportRequestToParentResponse,
    TransportError,
>;

#[derive(Debug)]
enum TransportContext {
    Bind {
        parent_msg: GhostMessage<
            TransportRequestToChild<Url>,
            TransportRequestToParent<Url>,
            TransportRequestToChildResponse<Url>,
            TransportError>,
    },
    SendMessage {
        parent_msg: GhostMessage<
            TransportRequestToChild<Url>,
            TransportRequestToParent<Url>,
            TransportRequestToChildResponse<Url>,
            TransportError>,
    },
}

/// Transport protocol enums for use with GhostActor implementation
#[derive(Debug)]
pub enum TransportRequestToChild<ADDR> {
    Bind { spec: ADDR }, // wss://0.0.0.0:0 -> all network interfaces first available port
    SendMessage { address: ADDR, payload: Vec<u8> },
}

#[derive(Debug)]
pub enum TransportRequestToChildResponse<ADDR> {
    Bind(BindResultData<ADDR>),
    SendMessage,
}

#[derive(Debug)]
pub enum TransportRequestToParent<ADDR> {
    IncomingConnection { address: ADDR },
    ReceivedData { address: ADDR, payload: Vec<u8> },
}

#[derive(Debug)]
pub enum TransportRequestToParentResponse {
    Allowed,    // just for testing
    Disallowed, // just for testing
}

#[derive(Debug)]
pub struct BindResultData<ADDR> {
    pub bound_url: ADDR,
}