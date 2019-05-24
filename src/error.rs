///! Error types related to MP3 decoding.

use std::io;

/// Error that can be raised during MP3 decoding.
#[derive(Debug)]
pub enum Error {
    /// An error during the MP3 decoding process.
    Mp3Error(Mp3Error),

    // An IO error reading the underlying stream.
    IoError(io::Error),
}

#[derive(Debug)]
pub enum Mp3Error {
    /// Invalid or unknown data was encountered when reading the stream.
    InvalidData(&'static str),

    /// An unsupported MP3 feature is used in this MP3 stream.
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
