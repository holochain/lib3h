use crate::transport::{error::TransportError, TransportId};

/// Commands that can be sent to an implementor of the Transport trait and handled during `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum TransportCommand {
    Connect(String),
    Send(Vec<TransportId>, Vec<u8>),
    SendAll(Vec<u8>),
    Close(TransportId),
    CloseAll,
    Bind(String),
}

/// Events that can be generated during a `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum TransportEvent {
    TransportError(TransportId, TransportError),
    ConnectResult(TransportId),
    Received(TransportId, Vec<u8>),
    Closed(TransportId),
}
