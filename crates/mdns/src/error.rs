#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MulticastDnsError {
    Generic(String),
}

impl std::error::Error for MulticastDnsError {
    fn description(&self) -> &str {
        "MulicastDnsError"
    }
}

impl std::fmt::Display for MulticastDnsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<std::io::Error> for MulticastDnsError {
    fn from(error: std::io::Error) -> Self {
        MulticastDnsError::Generic(format!("{:?}", error))
    }
}

impl From<std::option::NoneError> for MulticastDnsError {
    fn from(error: std::option::NoneError) -> Self {
        MulticastDnsError::Generic(format!("{:?}", error))
    }
}

impl From<std::net::AddrParseError> for MulticastDnsError {
    fn from(error: std::net::AddrParseError) -> Self {
        MulticastDnsError::Generic(format!("{:?}", error))
    }
}

pub type MulticastDnsResult<T> = Result<T, MulticastDnsError>;
