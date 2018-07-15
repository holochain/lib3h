/*!
Error / Result structure.

All libsodacrypt apis will return an error::Result.
*/

use std;

/**
Basic error structure for libsodacrypt.
*/
pub struct Error {
    error: Box<ErrorType>,
}

/**
Basic result type for libsodacrypt.
*/
pub type Result<T> = std::result::Result<T, Error>;

enum ErrorType {
    GenericError(Box<std::fmt::Debug>),
    IoError(std::io::Error),
}

impl Error {
    /**
    Generate an error struct based on a &str value.
    */
    pub fn str_error(s: &str) -> Self {
        Error {
            error: Box::new(ErrorType::GenericError(Box::new(format!("{}", s)))),
        }
    }

    /**
    Generate an error struct based of anything implementing std::fmt::Debug.
    */
    pub fn generic_error(e: Box<std::fmt::Debug>) -> Self {
        Error {
            error: Box::new(ErrorType::GenericError(e)),
        }
    }
}

impl<'a> From<&'a str> for Error {
    fn from(e: &'a str) -> Self {
        Error {
            error: Box::new(ErrorType::GenericError(Box::new(format!("{}", e)))),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error {
            error: Box::new(ErrorType::IoError(e)),
        }
    }
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ErrorType::GenericError(ref err) => f.write_str(&format!("{:?}", err)),
            ErrorType::IoError(ref err) => f.write_str(&format!("{:?}", err)),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "use Display"
    }

    fn cause(&self) -> Option<&std::error::Error> {
        None
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.error, f)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error({:?})", self.error.to_string())
    }
}
