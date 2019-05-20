//! Lib3h API SodiumError Enum

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SodiumError {
    Generic(String),
    OutputLength(String),
    OutOfMemory,
}

impl SodiumError {
    pub fn new(msg: &str) -> Self {
        SodiumError::Generic(msg.to_string())
    }
}

impl std::error::Error for SodiumError {
    fn description<'a>(&'a self) -> &'a str {
        "SodiumError"
    }
}

impl std::fmt::Display for SodiumError {
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
            &format!("{}", SodiumError::Generic("bla".to_string()))
        );
        assert_eq!(
            "OutputLength(\"bla\")",
            &format!("{}", SodiumError::OutputLength("bla".to_string()))
        );
        assert_eq!("OutOfMemory", &format!("{}", SodiumError::OutOfMemory));
    }
}
