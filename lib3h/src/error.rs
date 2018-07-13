use libsodacon;
use std;

pub struct Error {
    error: Box<ErrorType>,
}

pub type Result<T> = std::result::Result<T, Error>;

enum ErrorType {
    GenericError(Box<std::fmt::Debug>),
    SodaconError(libsodacon::error::Error),
}

impl Error {
    pub fn str_error(s: &str) -> Self {
        Error {
            error: Box::new(ErrorType::GenericError(Box::new(format!("{}", s))))
        }
    }

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

impl From<libsodacon::error::Error> for Error {
    fn from(e: libsodacon::error::Error) -> Self {
        Error {
            error: Box::new(ErrorType::SodaconError(e)),
        }
    }
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ErrorType::GenericError(ref err) => f.write_str(&format!("{:?}", err)),
            ErrorType::SodaconError(ref err) => f.write_str(&format!("{:?}", err)),
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
