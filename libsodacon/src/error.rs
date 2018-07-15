use hex;
use libsodacrypt;
use rmp_serde;
use std;

pub struct Error {
    error: Box<ErrorType>,
}

pub type Result<T> = std::result::Result<T, Error>;

enum ErrorType {
    GenericError(Box<std::fmt::Debug>),
    SodacryptError(libsodacrypt::error::Error),
    RmpDecode(rmp_serde::decode::Error),
    RmpEncode(rmp_serde::encode::Error),
    IoError(std::io::Error),
    AddrParseError(std::net::AddrParseError),
    FromHexError(hex::FromHexError),
}

impl Error {
    pub fn str_error(s: &str) -> Self {
        Error {
            error: Box::new(ErrorType::GenericError(Box::new(format!("{}", s)))),
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

impl From<libsodacrypt::error::Error> for Error {
    fn from(e: libsodacrypt::error::Error) -> Self {
        Error {
            error: Box::new(ErrorType::SodacryptError(e)),
        }
    }
}

impl From<rmp_serde::decode::Error> for Error {
    fn from(e: rmp_serde::decode::Error) -> Self {
        Error {
            error: Box::new(ErrorType::RmpDecode(e)),
        }
    }
}

impl From<rmp_serde::encode::Error> for Error {
    fn from(e: rmp_serde::encode::Error) -> Self {
        Error {
            error: Box::new(ErrorType::RmpEncode(e)),
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

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Self {
        Error {
            error: Box::new(ErrorType::AddrParseError(e)),
        }
    }
}

impl From<hex::FromHexError> for Error {
    fn from(e: hex::FromHexError) -> Self {
        Error {
            error: Box::new(ErrorType::FromHexError(e)),
        }
    }
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ErrorType::GenericError(ref err) => f.write_str(&format!("{:?}", err)),
            ErrorType::SodacryptError(ref err) => f.write_str(&format!("{:?}", err)),
            ErrorType::RmpDecode(ref err) => f.write_str(&format!("{:?}", err)),
            ErrorType::RmpEncode(ref err) => f.write_str(&format!("{:?}", err)),
            ErrorType::IoError(ref err) => f.write_str(&format!("{:?}", err)),
            ErrorType::AddrParseError(ref err) => f.write_str(&format!("{:?}", err)),
            ErrorType::FromHexError(ref err) => f.write_str(&format!("{:?}", err)),
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
