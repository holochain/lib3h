pub type GhostResult<T> = Result<T, GhostError>;

#[derive(Debug)]
pub struct GhostError(Box<ErrorKind>);

impl GhostError {
    /// A crate private constructor for `Error`.
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
#[derive(Debug)]
pub enum ErrorKind {
    Other(String),
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
            ErrorKind::Other(ref _s) => None,
            _ => unreachable!(),
        }
    }
}

impl std::fmt::Display for GhostError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self.0 {
            ErrorKind::Other(ref s) => write!(f, "Unknown error encountered: '{}'.", s),
            _ => unreachable!(),
        }
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