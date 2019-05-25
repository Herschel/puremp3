///! Error types related to MP3 decoding.
use std::{fmt, io};

/// Error that can be raised during MP3 decoding.
#[derive(Debug)]
pub enum Error {
    /// An error during the MP3 decoding process.
    Mp3Error(Mp3Error),

    // An IO error reading the underlying stream.
    IoError(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Mp3Error(e) => write!(f, "MP3 Error: {}", e),
            Error::IoError(e) => write!(f, "IO Error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Mp3Error(e) => Some(e),
            Error::IoError(e) => Some(e),
        }
    }
}

#[derive(Debug)]
pub enum Mp3Error {
    /// Invalid or unknown data was encountered when reading the stream.
    InvalidData(&'static str),

    /// An unsupported MP3 feature is used in this MP3 stream.
    Unsupported(&'static str),
}

impl fmt::Display for Mp3Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Mp3Error::InvalidData(s) => write!(f, "Invalid data: {}", s),
            Mp3Error::Unsupported(s) => write!(f, "Unsupported: {}", s),
        }
    }
}

impl std::error::Error for Mp3Error {}

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
