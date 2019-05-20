use std::io;

#[derive(Debug)]
pub enum Error {
    Mp3Error(Mp3Error),
    IoError(io::Error),
}

#[derive(Debug)]
pub enum Mp3Error {
    InvalidData(&'static str),
    Unsupported(&'static str),
}

impl From<Mp3Error> for Error {
    fn from(error: Mp3Error) -> Self {
        Error::Mp3Error(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IoError(error)
    }
}
