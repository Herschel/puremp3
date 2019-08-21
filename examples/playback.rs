//! MP3 playback example.
//!
//! Uses puremp3 for MP3 decoding, sample for sample format conversion,
//! and cpal for audio output.
//!
//! Usage: `playback file.mp3`
use cpal::{
    traits::{DeviceTrait, EventLoopTrait, HostTrait},
    StreamData, UnknownTypeOutputBuffer,
};
use sample::{interpolate, signal, Frame, Signal};

fn main() {
    // Load the input file.
    let mut args = std::env::args();
    let filename = if let Some(filename) = args.nth(1) {
        filename
    } else {
        eprintln!("usage: playback file");
        std::process::exit(-1);
    };

    // Playback the file and handle any errors.
    match playback(&filename) {
        Ok(()) => (),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(-1)
        }
    }
}

fn playback(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mp3_data = std::fs::read(filename)?;

    // Create the MP3 input stream.
    let (header, samples) = puremp3::read_mp3(&mp3_data[..])?;

    // Create the output audio stream using the cpal crate.
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("Failed to get default output device")?;
    let format_range = device
        .supported_output_formats()
        .unwrap()
        .next()
        .ok_or("Failed to get endpoint format")?;
    let mut out_format = format_range.with_max_sample_rate();
    out_format.sample_rate = cpal::SampleRate(44_100);
    let event_loop = host.event_loop();
    let stream_id = event_loop
        .build_output_stream(&device, &out_format)
        .expect("Failed to create a stream");
    event_loop.play_stream(stream_id.clone()).expect("Cannot play stream");

    // Use the sample crate to convert the MP3 stream to the output stream format.
    let mut signal = signal::from_iter(samples.map(|sample| [sample.0, sample.1]));
    let interp = interpolate::Linear::from_source(&mut signal);
    let mut signal = signal.from_hz_to_hz(
        interp,
        header.sample_rate.hz().into(),
        out_format.sample_rate.0.into(),
    );

    event_loop.run(move |stream_id, buffer| {
        let stream_data = match buffer {
            Ok(data) => data,
            Err(err) => {
                eprintln!("an error occurred on stream {:?}: {}", stream_id, err);
                return;
            }
        };

        match stream_data {
            StreamData::Output {
                buffer: UnknownTypeOutputBuffer::F32(mut buffer),
            } => write_samples(&mut signal, &out_format, &mut buffer),

            StreamData::Output {
                buffer: UnknownTypeOutputBuffer::I16(mut buffer),
            } => write_samples(&mut signal, &out_format, &mut buffer),

            StreamData::Output {
                buffer: UnknownTypeOutputBuffer::U16(mut buffer),
            } => write_samples(&mut signal, &out_format, &mut buffer),

            _ => unreachable!(),
        }
    });
}

/// Writes samples from the MP3 stream to the output audio stream.
/// Generic because we don't know the format of the output stream until runtime.
fn write_samples<S, T>(
    in_signal: &mut S,
    out_format: &cpal::Format,
    out_buffer: &mut cpal::OutputBuffer<'_, T>,
) where
    S: sample::Signal<Frame = [f32; 2]>,
    T: cpal::Sample + sample::Sample + sample::conv::FromSample<f32>,
{
    for out_sample in out_buffer.chunks_mut(out_format.channels as usize) {
        if in_signal.is_exhausted() {
            // Exit the app when the stream is complete.
            std::process::exit(0);
        }

        let in_sample: [T; 2] = in_signal.next().map(sample::Sample::to_sample);
        out_sample.copy_from_slice(&in_sample[..]);
    }
}
