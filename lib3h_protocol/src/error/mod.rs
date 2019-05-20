//! Lib3h API Error Enum

pub mod sodium_error;

use sodium_error::SodiumError;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lib3hError {
    SodiumError(SodiumError),
}

impl std::error::Error for Lib3hError {
    fn description<'a>(&'a self) -> &'a str {
        "Lib3hError"
    }
}

impl std::fmt::Display for Lib3hError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<SodiumError> for Lib3hError {
    fn from(error: SodiumError) -> Self {
        Lib3hError::SodiumError(error.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_display_types() {
        assert_eq!(
            "SodiumError(OutOfMemory)",
            &format!("{}", Lib3hError::from(SodiumError::OutOfMemory))
        );
    }
}
