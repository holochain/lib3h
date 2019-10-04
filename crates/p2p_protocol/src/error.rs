/// Result type for P2pErrors
pub type P2pResult<T> = Result<T, P2pError>;

/// P2pError used in GhostResult responses
#[derive(Debug, PartialEq)]
pub struct P2pError(Box<ErrorKind>);

impl P2pError {
    /// create a new `P2pError`.
    pub fn new(kind: ErrorKind) -> Self {
        P2pError(Box::new(kind))
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
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// Generic stringified errors
    Other(String),
    /// Hints that destructuring should not be exhaustive.
    ///
    /// This enum may grow additional variants, so this makes sure clients
    /// don't count on exhaustive matching. (Otherwise, adding a new variant
    /// could break existing code.)
    #[doc(hidden)]
    __Nonexhaustive,
}

impl std::error::Error for P2pError {
    /// The lower-level source of this error, if any.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self.0 {
            ErrorKind::Other(ref _s) => None,
            _ => unreachable!(),
        }
    }
}

impl std::fmt::Display for P2pError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self.0 {
            ErrorKind::Other(ref s) => write!(f, "Unknown error encountered: '{}'.", s),
            _ => unreachable!(),
        }
    }
}

impl From<String> for P2pError {
    fn from(s: String) -> Self {
        P2pError::new(ErrorKind::Other(s))
    }
}

impl From<&str> for P2pError {
    fn from(s: &str) -> Self {
        P2pError::new(ErrorKind::Other(s.to_string()))
    }
}

impl From<capnp::Error> for P2pError {
    fn from(e: capnp::Error) -> Self {
        format!("{:?}", e).into()
    }
}
