use std::fmt;
use std::io::Error as IoError;

#[derive(Debug)]
pub enum Error {
    Io(IoError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "{}", err),
        }
    }
}
