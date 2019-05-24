//! An MP3 decoder implemented in pure Rust.
//!
//! Supports MPEG-1, MPEG-2, and MPEG-2.5 Layer III streams.
//! Layers I and II are currently unsupported.
//!
//! # Example
//!
//! ```
//! let data = std::fs::read("tests/vectors/MonoCBR192.mp3").expect("Could not open file");
//! let decoder = puremp3::Mp3Decoder::new(&data[..]);
//! let (_, samples) = decoder.samples().expect("Invalid MP3");
//! for (left, right) in samples {
//!     // Operate on samples here
//! }
//! ```

mod decoder;
mod error;
mod huffman;
mod requantize;
mod stereo;
mod synthesis;
mod tables;

pub use crate::error::{Error, Mp3Error};

use std::io::Read;

/// Decodes MP3 streams.
pub struct Mp3Decoder<R: Read> {
    reader: R,
    state: decoder::DecoderState,
}

impl<R: Read> Mp3Decoder<R> {
    /// Creates a new `MP3Decoder` from the given reader.
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            state: decoder::DecoderState::new(),
        }
    }

    /// Gets a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    /// Gets a mutable reference to the underlying reader.
    ///
    /// It is inadvisable to directly read from the underlying reader.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    /// Unwraps the `Mp3Decoder`, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.reader
    }

    /// Returns an `Iterator` that yields MP3 `Frame`s.
    ///
    /// Each `Frame` contains header information and the decoded samples.
    /// Any invalid data is skipped. The iterator will provide `Frame`s until
    /// there is no more valid MP3 data or an error occurs.
    ///
    /// If you wish to inspect any errors, Use `next_frame` instead.
    pub fn frames(mut self) -> impl Iterator<Item = Frame> {
        std::iter::from_fn(move || self.next_frame().ok())
    }

    /// Returns an `Iterator` that yields MP3 `Sample`s`.
    ///
    /// This is a convenience function that assumes that the format of the MP3 does not
    /// change mid-stream.
    ///
    /// Each `Sample` represents one left and right sample at the sample rate of the MP3.
    /// Any invalid data is skipped. The iterator will provide `Sample`s until
    /// there is no more valid MP3 data, or an error occurs.
    pub fn samples(mut self) -> Option<(decoder::FrameHeader, impl Iterator<Item = (f32, f32)>)> {
        let mut frame = self.next_frame().ok()?;
        let header = frame.header.clone();
        let mut i = 0;
        let iter = std::iter::from_fn(move || {
            if i >= frame.num_samples {
                i = 0;
                frame = if let Ok(frame) = self.next_frame() {
                    frame
                } else {
                    return None;
                }
            }
            let sample = (frame.samples[0][i], frame.samples[1][i]);
            i += 1;
            Some(sample)
        });
        Some((header, iter))
    }

    /// Decodes the next MP3 `Frame` in the stream.
    ///
    /// Data is read until a valid `Frame` is found. Invalid data is skipped.
    /// Other errors are returned.
    pub fn next_frame(&mut self) -> Result<Frame, Error> {
        let header;
        loop {
            match decoder::read_frame_header(&mut self.reader) {
                Ok(frame_header) => {
                    header = frame_header;
                    break;
                }
                Err(Error::Mp3Error(Mp3Error::InvalidData(_))) => (),
                Err(e) => return Err(e),
            }
        }

        let (num_samples, samples) =
            decoder::process_frame(&mut self.state, &mut self.reader, &header)?;

        Ok(Frame {
            header,
            samples,
            num_samples,
        })
    }
}

/// A frame of MP3 data.
///
/// Each frame contains a header describing the format of the data, and the decoded
/// samples. An MP3 frame contains either 576 or 1152 samples (depending on the
/// format).
pub struct Frame {
    pub header: decoder::FrameHeader,
    pub samples: [[f32; 1152]; 2],
    pub num_samples: usize,
}

/// A sample of sound data in the range [-1.0, 1.0].
///
/// Contains values for the left and right channel. In mono streams, the sample
/// is duplicated for both channels.
pub struct Sample {
    pub header: decoder::FrameHeader,
    pub sample: (f32, f32),
}
