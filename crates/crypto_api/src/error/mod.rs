//! Lib3h Crypto API CryptoError module

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CryptoError {
    Generic(String),
    OutputLength(String),
    OutOfMemory,
}

impl CryptoError {
    pub fn new(msg: &str) -> Self {
        CryptoError::Generic(msg.to_string())
    }
}

impl std::error::Error for CryptoError {
    fn description(&self) -> &str {
        "CryptoError"
    }
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_display_types() {
        assert_eq!(
            "Generic(\"bla\")",
            &format!("{}", CryptoError::Generic("bla".to_string()))
        );
        assert_eq!(
            "OutputLength(\"bla\")",
            &format!("{}", CryptoError::OutputLength("bla".to_string()))
        );
        assert_eq!("OutOfMemory", &format!("{}", CryptoError::OutOfMemory));
    }
}
