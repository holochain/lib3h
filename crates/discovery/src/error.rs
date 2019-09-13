//! mDNS module error definition.

use std::{error::Error as StdError, fmt, io};

pub type DiscoveryResult<T> = Result<T, DiscoveryError>;

/// An error that can occur while discovery participants on a network.
// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Debug)]
pub struct DiscoveryError(Box<ErrorKind>);

impl DiscoveryError {
    /// A constructor for `DiscoveryError`.
    pub fn new(kind: ErrorKind) -> Self {
        DiscoveryError(Box::new(kind))
    }

    /// Helper function to build a new error with an [Other](ErrorKind::Other) ErrorKind.
    pub fn new_other(s: &str) -> Self {
        DiscoveryError::new(ErrorKind::Other(s.to_owned()))
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

impl StdError for DiscoveryError {
    /// The lower-level source of this error, if any.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self.0 {
            ErrorKind::Io(ref err) => Some(err),
            ErrorKind::Other(ref _s) => None,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self.0 {
            ErrorKind::Io(ref err) => err.fmt(f),
            ErrorKind::Other(ref s) => write!(f, "Unknown error encountered: '{}'.", s),
            _ => unreachable!(),
        }
    }
}

impl From<io::Error> for DiscoveryError {
    fn from(err: io::Error) -> Self {
        DiscoveryError::new(ErrorKind::Io(err))
    }
}
