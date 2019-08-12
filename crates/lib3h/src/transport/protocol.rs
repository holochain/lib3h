use crate::transport::{error::TransportError, ConnectionId};
use url::Url;
use lib3h_protocol::Address;

#[derive(Debug, Clone, PartialEq)]
pub struct SendData {
    pub id_list: Vec<ConnectionId>,
    pub payload: Vec<u8>,
    pub request_id: Option<String>,
    /// TODO XXX - HACK until we have channels
    pub chain_id: Option<(Address, Address)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SuccessResultData {
    pub request_id: String,
    /// TODO XXX - HACK until we have channels
    pub chain_id: Option<(Address, Address)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FailureResultData {
    pub request_id: String,
    pub error: Vec<TransportError>,
    /// TODO XXX - HACK until we have channels
    pub chain_id: Option<(Address, Address)>,
}

/// Commands that can be sent to an implementor of the Transport trait and handled during `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum TransportCommand {
    Connect(Url, /*request_id*/ String),
    SendReliable(SendData),
    SendAll(Vec<u8>),
    Close(ConnectionId),
    CloseAll,
    Bind(Url),
}

/// Events that can be generated during a `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum TransportEvent {
    /// Result indicating a generic request success
    SuccessResult(SuccessResultData),
    /// Result indicating a generic request failure
    FailureResult(FailureResultData),
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
