use crate::transport::error::TransportError;
use lib3h_protocol::Address;
use url::Url;

pub type RequestId = String;

/// Commands that can be sent to an implementor of the Transport trait and handled during `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum TransportCommand {
    /// establish a connection point to receive incoming connections
    Bind { request_id: RequestId, spec: Url },
    /// establish a connection with a remote node
    Connect { request_id: RequestId, address: Url },
    /// send a message to a remote node, may first establish a connection
    SendMessage {
        request_id: RequestId,
        address: Url,
        payload: Vec<u8>,
    },
}

/// Events that can be generated during a `process()`
#[derive(Debug, PartialEq, Clone)]
pub enum TransportEvent {
    /// Any TransportCommand can asyncronously generate a FailureResult
    FailureResult {
        request_id: RequestId,
        error: TransportError,
    },
    /// Success response to TransportCommand::Bind
    BindSuccess {
        request_id: RequestId,
        bound_address: Url,
    },
    /// Success response to TransportCommand::Connect
    ConnectSuccess { request_id: RequestId, address: Url },
    /// Success response to TransportCommand::SendMessage
    SendMessageSuccess { request_id: RequestId },
    /// An incoming connection has been established
    IncomingConnection { address: Url },
    /// We have received data from a connection
    ReceivedData { address: Url, payload: Vec<u8> },
    /// We encountered an error with a specific connection
    ConnectionError { address: Url, error: TransportError },
    /// A connection closed for whatever reason
    ConnectionClosed { address: Url },
}
