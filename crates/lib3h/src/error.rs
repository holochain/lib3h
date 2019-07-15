//! Lib3h custom error definition.

use crate::transport::error::TransportError;
use lib3h_crypto_api::CryptoError;
use lib3h_protocol::error::{ErrorKind as Lib3hProtocolErrorKind, Lib3hProtocolError};
use rmp_serde::decode::Error as RMPSerdeDecodeError;
use std::{error::Error as StdError, fmt, io, result};

/// A type alias for `Result<T, Lib3hError>`.
pub type Lib3hResult<T> = result::Result<T, Lib3hError>;

/// An error that can occur when interacting with the algorithm.
#[derive(Debug)]
pub struct Lib3hError(Box<ErrorKind>);

impl Lib3hError {
    /// A constructor for `Lib3hError`.
    pub fn new(kind: ErrorKind) -> Lib3hError {
        Lib3hError(Box::new(kind))
    }

    /// Helper function to build a new error with an [Other](ErrorKind::Other) ErrorKind.
    pub fn new_other(s: &str) -> Self {
        Lib3hError::new(ErrorKind::Other(s.to_owned()))
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
    TransportError(TransportError),
    /// Error originating from [lib3h_protocol] crate.
    Lib3hProtocolError(Lib3hProtocolError),
    /// Error occuring from [Hcid](hcid) crate.
    HcId(hcid::HcidError),
    /// Error originating from [MessagePack](rmp_serde) deserializing crate.
    RmpSerdeDecodeError(RMPSerdeDecodeError),
    /// Error from the [lib3h_crypto_api] crate.
    CryptoApiError(CryptoError),
    /// Error occuring when the key is not present in the Map.
    KeyNotFound(String),
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

impl StdError for Lib3hError {
    /// The lower-level source of this error, if any.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self.0 {
            ErrorKind::Io(ref err) => Some(err),
            ErrorKind::TransportError(ref err) => Some(err),
            ErrorKind::Lib3hProtocolError(ref err) => Some(err),
            ErrorKind::HcId(ref err) => Some(err),
            ErrorKind::RmpSerdeDecodeError(ref err) => Some(err),
            ErrorKind::CryptoApiError(ref err) => Some(err),
            ErrorKind::Other(ref _s) | ErrorKind::KeyNotFound(ref _s) => None,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for Lib3hError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self.0 {
            ErrorKind::Io(ref err) => err.fmt(f),
            ErrorKind::TransportError(ref err) => err.fmt(f),
            ErrorKind::Lib3hProtocolError(ref err) => err.fmt(f),
            ErrorKind::HcId(ref err) => err.fmt(f),
            ErrorKind::RmpSerdeDecodeError(ref err) => err.fmt(f),
            ErrorKind::CryptoApiError(ref err) => err.fmt(f),
            ErrorKind::KeyNotFound(ref s) => write!(f, "Key: '{}' not found", s),
            ErrorKind::Other(ref s) => write!(f, "Unknown error encountered: '{}'.", s),
            _ => unreachable!(),
        }
    }
}

impl From<io::Error> for Lib3hError {
    fn from(err: io::Error) -> Self {
        Lib3hError::new(ErrorKind::Io(err))
    }
}

impl From<TransportError> for Lib3hError {
    fn from(err: TransportError) -> Self {
        Lib3hError::new(ErrorKind::TransportError(err))
    }
}

impl From<hcid::HcidError> for Lib3hError {
    fn from(err: hcid::HcidError) -> Self {
        Lib3hError::new(ErrorKind::HcId(err))
    }
}

impl From<Lib3hProtocolError> for Lib3hError {
    fn from(err: Lib3hProtocolError) -> Self {
        Lib3hError::new(ErrorKind::Lib3hProtocolError(err))
    }
}

impl From<RMPSerdeDecodeError> for Lib3hError {
    fn from(err: RMPSerdeDecodeError) -> Self {
        Lib3hError::new(ErrorKind::RmpSerdeDecodeError(err))
    }
}

impl From<CryptoError> for Lib3hError {
    fn from(err: CryptoError) -> Self {
        Lib3hError::new(ErrorKind::CryptoApiError(err))
    }
}

// I'm not so sure about this...
impl From<Lib3hError> for Lib3hProtocolError {
    fn from(_err: Lib3hError) -> Self {
        Lib3hProtocolError::new(Lib3hProtocolErrorKind::Other(String::from(
            "Lib3hProtocolError occuring in Lib3h.",
        )))
    }
}
