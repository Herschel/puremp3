//! An MP3 decoder implemented in pure Rust.
//!
//! Supports MPEG-1, MPEG-2, and MPEG-2.5 Layer III streams.
//! Layers I and II are currently unsupported.
//!
//! # Example
//!
//! ```
//! let data = std::fs::read("tests/vectors/MonoCBR192.mp3").expect("Could not open file");
//! println!("{}", data.len());
//! let (header, samples) = puremp3::read_mp3(&data[..]).expect("Invalid MP3");
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
mod types;

pub use crate::error::{Error, Mp3Error};
pub use crate::types::{
    BitRate, Channels, Emphasis, FrameHeader, MpegLayer, MpegVersion, SampleRate,
};

use std::io::Read;

/// Convenience method to decode an MP3.
/// Returns the first frame header found in the MP3, and an `Iterator` that
/// yields MP3 `Sample`s`.
///
/// Each `Sample` represents one left and right sample at the sample rate of
/// the MP3. Any invalid data is ignored. The iterator will provide `Sample`s
/// until there is no more data, or an error occurs.
///
/// If you need to handle errors or changes in the format mid-stream, use
/// `Mp3Decoder` driectly.
pub fn read_mp3<R: Read>(
    reader: R,
) -> Result<(FrameHeader, impl Iterator<Item = (f32, f32)>), Error> {
    let mut decoder = Mp3Decoder::new(reader);
    let mut frame = decoder.next_frame()?;
    let header = frame.header.clone();
    let mut i = 0;
    let iter = std::iter::from_fn(move || {
        if i >= frame.num_samples {
            i = 0;
            frame = if let Ok(frame) = decoder.next_frame() {
                frame
            } else {
                return None;
            }
        }
        let sample = (frame.samples[0][i], frame.samples[1][i]);
        i += 1;
        Some(sample)
    });
    Ok((header, iter))
}

/// Decodes MP3 streams.
pub struct Mp3Decoder<R: Read> {
    reader: R,
    state: crate::types::DecoderState,
}

impl<R: Read> Mp3Decoder<R> {
    /// Creates a new `MP3Decoder` from the given reader.
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            state: crate::types::DecoderState::new(),
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
    /// The header of this MP3 frame.
    pub header: FrameHeader,

    /// The decoded MP3 samples for the left and right channels.
    /// Each sample is in the range of [-1.0, 1.0].
    /// Only the first `num_samples` entries will contain valid data.
    /// For mono streams, the data will be duplicated to the left and right
    /// channels.
    pub samples: [[f32; 1152]; 2],

    /// The number of samples in the `samples` array.
    /// This will be either 576 or 1152 samples depending on the
    /// format of the MP3.
    pub num_samples: usize,
}
