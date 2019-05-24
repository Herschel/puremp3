mod decoder;
mod error;
mod huffman;
mod requantize;
mod stereo;
mod synthesis;
mod tables;

use crate::error::{Error, Mp3Error};
use std::io::Read;

pub struct Mp3Decoder<R: Read> {
    reader: R,
    state: decoder::DecoderState,
}

impl<R: Read> Mp3Decoder<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            state: decoder::DecoderState::new(),
        }
    }

    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    pub fn into_inner(self) -> R {
        self.reader
    }

    pub fn frames(mut self) -> impl Iterator<Item = Frame> {
        std::iter::from_fn(move || self.next_frame().ok())
    }

    pub fn samples(mut self) -> Option<(decoder::FrameHeader, impl Iterator<Item = (f32, f32)>)> {
        let mut frame = self.next_frame().ok()?;
        let header = frame.header.clone();
        let mut i = 0;
        let iter = std::iter::from_fn(move || {
            if i >= frame.num_samples {
                i = 0;
                frame = if let Ok(frame) = self.next_frame() {
                    frame
                } else { return None; }
            }
            let sample = (frame.samples[0][i], frame.samples[1][i]);
            i += 1;
            Some(sample)
        });
        Some((header, iter))
    }

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

pub struct Frame {
    pub header: decoder::FrameHeader,
    pub samples: [[f32; 1152]; 2],
    pub num_samples: usize,
}

pub struct Sample {
    pub header: decoder::FrameHeader,
    pub sample: (f32, f32),
}
