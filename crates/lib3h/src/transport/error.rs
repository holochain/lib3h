//! Connection Error struct and TransportResult type

use tungstenite::handshake::{
    server::{NoCallback, ServerHandshake},
    HandshakeError,
};

/// a result object whos error is a TransportError instance
pub type TransportResult<T> = Result<T, TransportError>;

/// represents an error generated by a connection instance
#[derive(Debug, PartialEq, Clone)]
pub struct TransportError(Box<ErrorKind>);

impl TransportError {
    /// create a new `TransportError`.
    pub fn new(e: String) -> Self {
        TransportError(Box::new(ErrorKind::Other(e)))
    }

    /// create a new `TransportError`.
    pub fn new_kind(kind: ErrorKind) -> Self {
        TransportError(Box::new(kind))
    }

    /// Return the specific type of this error.
    pub fn kind(&self) -> &ErrorKind {
        &self.0
    }

    /// Unwrap this error into its underlying type.
    pub fn into_kind(self) -> ErrorKind {
        *self.0
    }
}

/// The specific type of an error.
#[derive(Debug, PartialEq, Clone)]
pub enum ErrorKind {
    Unbind,
    Other(String),
    /// Hints that destructuring should not be exhaustive.
    ///
    /// This enum may grow additional variants, so this makes sure clients
    /// don't count on exhaustive matching. (Otherwise, adding a new variant
    /// could break existing code.)
    #[doc(hidden)]
    __Nonexhaustive,
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self.0 {
            ErrorKind::Unbind => write!(f, "Unbind"),
            ErrorKind::Other(ref s) => write!(f, "{}", s),
            _ => unreachable!(),
        }
    }
}

impl std::error::Error for TransportError {
    /// The lower-level source of this error, if any.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self.0 {
            ErrorKind::Unbind => None,
            ErrorKind::Other(ref _s) => None,
            _ => unreachable!(),
        }
    }
}

impl From<String> for TransportError {
    fn from(s: String) -> Self {
        TransportError::new_kind(ErrorKind::Other(s))
    }
}

impl From<&str> for TransportError {
    fn from(s: &str) -> Self {
        TransportError::new_kind(ErrorKind::Other(s.to_string()))
    }
}

impl From<lib3h_ghost_actor::GhostError> for TransportError {
    fn from(error: lib3h_ghost_actor::GhostError) -> Self {
        Self::new(format!("{:?}", error))
    }
}

impl From<Vec<TransportError>> for TransportError {
    fn from(errors: Vec<TransportError>) -> Self {
        Self::new(format!("{:?}", errors))
    }
}

impl From<url::ParseError> for TransportError {
    fn from(error: url::ParseError) -> Self {
        Self::new(format!("{:?}", error))
    }
}

impl From<std::io::Error> for TransportError {
    fn from(error: std::io::Error) -> Self {
        Self::new(format!("{:?}", error))
    }
}

impl From<tungstenite::Error> for TransportError {
    fn from(error: tungstenite::Error) -> Self {
        Self::new(format!("{:?}", error))
    }
}

impl From<native_tls::Error> for TransportError {
    fn from(error: native_tls::Error) -> Self {
        Self::new(format!("{:?}", error))
    }
}

impl<S: std::fmt::Debug + std::io::Read + std::io::Write>
    From<HandshakeError<ServerHandshake<S, NoCallback>>> for TransportError
{
    fn from(error: HandshakeError<ServerHandshake<S, NoCallback>>) -> Self {
        Self::new(format!("{:?}", error))
    }
}

impl<T: std::io::Read + std::io::Write + std::fmt::Debug> From<native_tls::HandshakeError<T>>
    for TransportError
{
    fn from(error: native_tls::HandshakeError<T>) -> Self {
        Self::new(format!("{:?}", error))
    }
}

impl<T: std::io::Read + std::io::Write + std::fmt::Debug>
    From<tungstenite::HandshakeError<tungstenite::ClientHandshake<T>>> for TransportError
{
    fn from(error: tungstenite::HandshakeError<tungstenite::ClientHandshake<T>>) -> Self {
        Self::new(format!("{:?}", error))
    }
}

impl From<lib3h_protocol::error::Lib3hProtocolError> for TransportError {
    fn from(err: lib3h_protocol::error::Lib3hProtocolError) -> Self {
        Self::new(format!("{:?}", err))
    }
}

use lib3h_protocol::error::{ErrorKind as Lib3hErrorKind, Lib3hProtocolError};
impl From<TransportError> for Lib3hProtocolError {
    fn from(err: TransportError) -> Self {
        Self::new(Lib3hErrorKind::TransportError(format!("{:?}", err)))
    }
}
