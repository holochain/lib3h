//! Lib3h custom error definition. This is a zombie and it's bond to disapear.

/// Result type for GhostErrors
pub type GhostResult<T> = Result<T, GhostError>;

/// GhostError used in GhostResult responses
#[derive(Debug, Clone, PartialEq)]
pub struct GhostError(Box<ErrorKind>);

impl GhostError {
    /// create a new `GhostError`.
    pub fn new(kind: ErrorKind) -> Self {
        GhostError(Box::new(kind))
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
    /// If we have multiple sub-errors, we can represent them as a super-error
    Multiple(Vec<GhostError>),
    /// returned on an attempt to handle an callback for a non-existent request
    RequestIdNotFound(String),
    // /// Error occuring after a timeout.
    // Timeout(Backtwrap),
    /// Generic stringified errors
    Other(String),
    EndpointDisconnected,
    /// Hints that destructuring should not be exhaustive.
    ///
    /// This enum may grow additional variants, so this makes sure clients
    /// don't count on exhaustive matching. (Otherwise, adding a new variant
    /// could break existing code.)
    #[doc(hidden)]
    __Nonexhaustive,
}

impl std::error::Error for GhostError {
    /// The lower-level source of this error, if any.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self.0 {
            _ => None,
        }
    }
}

impl std::fmt::Display for GhostError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self.0 {
            ErrorKind::Multiple(ref s) => write!(f, "Multiple {{{:?}}}", s),
            ErrorKind::RequestIdNotFound(ref s) => write!(f, "RequestIdNotFound {{{:?}}}", s),
            ErrorKind::Other(ref s) => write!(f, "Unknown error encountered: '{}'.", s),
            _ => unreachable!(),
        }
    }
}

impl From<ErrorKind> for GhostError {
    fn from(k: ErrorKind) -> Self {
        GhostError::new(k)
    }
}

impl From<Vec<GhostError>> for GhostError {
    fn from(m: Vec<GhostError>) -> Self {
        GhostError::new(ErrorKind::Multiple(m))
    }
}

impl From<String> for GhostError {
    fn from(s: String) -> Self {
        GhostError::new(ErrorKind::Other(s))
    }
}

impl From<&str> for GhostError {
    fn from(s: &str) -> Self {
        GhostError::new(ErrorKind::Other(s.to_string()))
    }
}

impl<T> From<crossbeam_channel::SendError<T>> for GhostError {
    fn from(e: crossbeam_channel::SendError<T>) -> Self {
        GhostError::new(ErrorKind::Other(format!("{:?}", e)))
    }
}
