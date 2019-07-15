//! Lib3h_protocol custom error definition.

use std::{error::Error as StdError, fmt, io, result};
use serde::de::value::Error as DeserializeError;

/// A type alias for `Result<T, Lib3hProtocolError>`.
pub type Lib3hProtocolResult<T> = result::Result<T, Lib3hProtocolError>;

/// An error that can occur when interacting with the algorithm.
#[derive(Debug)]
pub struct Lib3hProtocolError(Box<ErrorKind>);

impl Lib3hProtocolError {
    /// A crate private constructor for `Error`.
    pub fn new(kind: ErrorKind) -> Lib3hProtocolError {
        Lib3hProtocolError(Box::new(kind))
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
#[derive(Debug)]
pub enum ErrorKind {
    /// An I/O error that occurred while processing a data stream.
    Io(io::Error),
    /// Error occuring when using `transport`.
    TransportError(String),
    /// An error occuring whiling trying to deserialize stuff during gossiping for example.
    DeserializeError(DeserializeError),
    /// Yet undefined error.
    Other(String),
    /// Hints that destructuring should not be exhaustive.
    ///
    /// This enum may grow additional variants, so this makes sure clients
    /// don't count on exhaustive matching. (Otherwise, adding a new variant
    /// could break existing code.)
    #[doc(hidden)]
    __Nonexhaustive,
}

impl StdError for Lib3hProtocolError {
    /// The lower-level source of this error, if any.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self.0 {
            ErrorKind::Io(ref err) => Some(err),
            ErrorKind::DeserializeError(ref err) => Some(err),
            ErrorKind::Other(ref _s) | ErrorKind::TransportError(ref _s) => None,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for Lib3hProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self.0 {
            ErrorKind::Io(ref err) => err.fmt(f),
            ErrorKind::TransportError(ref s) => write!(f, "TransportError: '{}'.", s),
            ErrorKind::DeserializeError(ref err) => err.fmt(f),
            ErrorKind::Other(ref s) => write!(f, "Unknown error encountered: '{}'.", s),
            _ => unreachable!(),
        }
    }
}

impl From<io::Error> for Lib3hProtocolError {
    fn from(err: io::Error) -> Self {
        Lib3hProtocolError::new(ErrorKind::Io(err))
    }
}

impl From<DeserializeError> for Lib3hProtocolError {
    fn from(err: DeserializeError) -> Self {
        Lib3hProtocolError::new(ErrorKind::DeserializeError(err))
    }
}

// impl From<hcid::HcidError> for Lib3hProtocolError {
//     fn from(err: hcid::HcidError) -> Self {
//         Lib3hProtocolError::new(ErrorKind::HcId(err))
//     }
// }

// impl From<Lib3hProtocolError> for io::Error {
//     fn from(err: Lib3hProtocolError) -> Self {
//         io::Error::new(io::ErrorKind::Other, err)
//     }
// }

// impl From<transport::error::TransportError> for Lib3hProtocolError {
//     fn from(err: transport::error::TransportError) -> Self {
//         Lib3hProtocolError::new(ErrorKind::(err))
//     }
// }

