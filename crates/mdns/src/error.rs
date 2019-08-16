//! mDNS module error definition.

use regex;
use std::{error::Error as StdError, fmt, io, net};

pub type MulticastDnsResult<T> = Result<T, MulticastDnsError>;

/// An error that can occur when interacting with the algorithm.
// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Debug)]
pub struct MulticastDnsError(Box<ErrorKind>);

impl MulticastDnsError {
    /// A constructor for `MulticastDnsError`.
    pub fn new(kind: ErrorKind) -> Self {
        MulticastDnsError(Box::new(kind))
    }

    /// Helper function to build a new error with an [Other](ErrorKind::Other) ErrorKind.
    pub fn new_other(s: &str) -> Self {
        MulticastDnsError::new(ErrorKind::Other(s.to_owned()))
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
    /// Error occuring from None Option
    NoneError(std::option::NoneError),
    /// Error occuring while parsing Adresses with the net module
    AddrParseError(net::AddrParseError),
    /// Error during probe.
    ProbeError,
    /// Error occuring while using Regex crate.
    RegexError(regex::Error),
    /// Error occuring while converting bytes to String.
    Utf8Error(std::str::Utf8Error),
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

impl StdError for MulticastDnsError {
    /// The lower-level source of this error, if any.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self.0 {
            ErrorKind::Io(ref err) => Some(err),
            ErrorKind::NoneError(ref _err) => None,
            ErrorKind::AddrParseError(ref err) => Some(err),
            ErrorKind::Utf8Error(ref err) => Some(err),
            ErrorKind::Other(ref _s) => None,
            ErrorKind::ProbeError => None,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for MulticastDnsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self.0 {
            ErrorKind::Io(ref err) => err.fmt(f),
            ErrorKind::NoneError(ref _err) => write!(f, "None value encountered."),
            ErrorKind::AddrParseError(ref err) => err.fmt(f),
            ErrorKind::Utf8Error(ref err) => err.fmt(f),
            ErrorKind::ProbeError => write!(f, "Error during probe."),
            ErrorKind::Other(ref s) => write!(f, "Unknown error encountered: '{}'.", s),
            _ => unreachable!(),
        }
    }
}

impl From<io::Error> for MulticastDnsError {
    fn from(err: io::Error) -> Self {
        MulticastDnsError::new(ErrorKind::Io(err))
    }
}

impl From<std::option::NoneError> for MulticastDnsError {
    fn from(err: std::option::NoneError) -> Self {
        MulticastDnsError::new(ErrorKind::NoneError(err))
    }
}

impl From<net::AddrParseError> for MulticastDnsError {
    fn from(err: net::AddrParseError) -> Self {
        MulticastDnsError::new(ErrorKind::AddrParseError(err))
    }
}

impl From<regex::Error> for MulticastDnsError {
    fn from(err: regex::Error) -> Self {
        MulticastDnsError::new(ErrorKind::RegexError(err))
    }
}

impl From<std::str::Utf8Error> for MulticastDnsError {
    fn from(err: std::str::Utf8Error) -> Self {
        MulticastDnsError::new(ErrorKind::Utf8Error(err))
    }
}
