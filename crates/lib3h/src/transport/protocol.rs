use crate::transport::{error::TransportError, ConnectionId};
use url::Url;

/// Commands that can be sent to an implementor of the Transport trait and handled during `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum TransportCommand {
    Connect(Url),
    Send(Vec<ConnectionId>, Vec<u8>),
    SendAll(Vec<u8>),
    Close(ConnectionId),
    CloseAll,
    Bind(Url),
}

/// Events that can be generated during a `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum TransportEvent {
    TransportError(ConnectionId, TransportError),
    /// an outgoing connection has been established
    ConnectResult(ConnectionId),
    /// we have received an incoming connection
    Connection(ConnectionId),
    Received(ConnectionId, Vec<u8>),
    Closed(ConnectionId),
}
