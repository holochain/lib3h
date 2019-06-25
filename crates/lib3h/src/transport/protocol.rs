use crate::transport::{error::TransportError, TransportId};
use url::Url;

/// Commands that can be sent to an implementor of the Transport trait and handled during `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum TransportCommand {
    Connect(Url),
    Send(Vec<TransportId>, Vec<u8>),
    SendAll(Vec<u8>),
    Close(TransportId),
    CloseAll,
    Bind(Url),
}

/// Events that can be generated during a `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum TransportEvent {
    TransportError(TransportId, TransportError),
    /// an outgoing connection has been established
    ConnectResult(TransportId),
    /// we have received an incoming connection
    Connection(TransportId),
    Received(TransportId, Vec<u8>),
    Closed(TransportId),
}
