mod decoder;
mod error;
mod huffman;
mod requantize;
mod stereo;
mod synthesis;
mod tables;

use crate::error::{Error, Mp3Error};
use std::io::Read;

pub struct Mp3Iterator<R: Read> {
    reader: R,
    decoder: decoder::Decoder,
}

impl<R: Read> Mp3Iterator<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            decoder: decoder::Decoder::new(),
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
}

pub struct Frame {
    pub header: decoder::FrameHeader,
    pub samples: [[f32; 1152]; 2],
    pub num_samples: usize,
}

impl<R: Read> Iterator for Mp3Iterator<R> {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        let header;
        loop {
            match decoder::read_frame_header(&mut self.reader) {
                Ok(frame_header) => {
                    header = frame_header;
                    break;
                }
                Err(Error::Mp3Error(Mp3Error::InvalidData(_))) => (),
                Err(_) => return None,
            }
        }

        let (num_samples, samples) = decoder::process_frame(&mut self.decoder, &mut self.reader, &header).ok()?;
        Some(Frame { header, samples, num_samples })
    }
}
