use openssl;
use rmp_serde;
use std;

pub struct Error {
    error: Box<ErrorType>,
}

pub type Result<T> = std::result::Result<T, Error>;

enum ErrorType {
    GenericError(Box<std::fmt::Debug>),
    OpenSslErrorStack(openssl::error::ErrorStack),
    RmpDecode(rmp_serde::decode::Error),
    RmpEncode(rmp_serde::encode::Error),
    IoError(std::io::Error),
}

impl Error {
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

impl From<openssl::error::ErrorStack> for Error {
    fn from(e: openssl::error::ErrorStack) -> Self {
        Error {
            error: Box::new(ErrorType::OpenSslErrorStack(e)),
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

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ErrorType::GenericError(ref err) => f.write_str(&format!("{:?}", err)),
            ErrorType::OpenSslErrorStack(ref err) => f.write_str(&format!("{:?}", err)),
            ErrorType::RmpDecode(ref err) => f.write_str(&format!("{:?}", err)),
            ErrorType::RmpEncode(ref err) => f.write_str(&format!("{:?}", err)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_generic_should_display() {
        let e = Error::from("hello");
        let res = format!("{}", e);
        assert_eq!("\"hello\"", res);
    }

    #[test]
    fn it_should_question_mark() {
        fn gen_err_1() -> Result<()> {
            Err(Error::from("hello"))
        }

        fn gen_err_2() -> Result<()> {
            gen_err_1()?;
            Ok(())
        }

        let error = gen_err_2().unwrap_err();
        let res = format!("{}", error);
        assert_eq!("\"hello\"", res);
    }

    #[test]
    fn it_should_assume_other_errors() {
        let bad = vec![0, 159, 146, 150];
        let sub = std::str::from_utf8(&bad).unwrap_err();
        let error = Error::generic_error(Box::new(sub));
        let res = format!("{}", error);
        assert_eq!("Utf8Error { valid_up_to: 1, error_len: Some(1) }", res);
    }

    #[derive(Debug)]
    enum TestError {
        MyTestError,
    }

    #[test]
    fn it_should_assume_custom_errors() {
        let error = Error::generic_error(Box::new(TestError::MyTestError));
        let res = format!("{}", error);
        assert_eq!("MyTestError", res);
    }
}
