//! MP3 playback example.
//!
//! Uses puremp3 for MP3 decoding, sample for sample format conversion,
//! and cpal for audio output.
//!
//! Usage: `playback file.mp3`
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

fn playback(filename: &str) -> Result<(), Box<std::error::Error>> {
    let mp3_data = std::fs::read(filename)?;

    // Create the MP3 input stream.
    let (header, samples) = puremp3::read_mp3(&mp3_data[..])?;

    // Create the output audio stream using the cpal crate.
    let device = cpal::default_output_device().ok_or("Failed to get default output device")?;
    let out_format = device.default_output_format()?;
    let event_loop = cpal::EventLoop::new();
    let stream_id = event_loop.build_output_stream(&device, &out_format)?;
    event_loop.play_stream(stream_id.clone());

    // Use the sample crate to convert the MP3 stream to the output stream format.
    let mut signal = signal::from_iter(samples.map(|sample| [sample.0, sample.1]));
    let interp = interpolate::Linear::from_source(&mut signal);
    let mut signal = signal.from_hz_to_hz(
        interp,
        header.sample_rate.hz().into(),
        out_format.sample_rate.0.into(),
    );

    // Run the stream.
    use cpal::{StreamData, UnknownTypeOutputBuffer};
    event_loop.run(move |_, data| match data {
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
