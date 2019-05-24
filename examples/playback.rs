use puremp3::Mp3Decoder;
use sample::interpolate::Linear;
use sample::{signal, Frame, Sample, Signal};

fn main() -> Result<(), Box<std::error::Error>> {
    let mut args = std::env::args();
    let input_path = args.nth(1).ok_or("Input file required")?;
    let mp3_data = std::fs::read(input_path)?;

    let device = cpal::default_output_device().ok_or("Failed to get default output device")?;
    let format = device.default_output_format()?;
    let event_loop = cpal::EventLoop::new();
    let stream_id = event_loop.build_output_stream(&device, &format)?;
    event_loop.play_stream(stream_id.clone());

    let (header, samples) = Mp3Decoder::new(std::io::Cursor::new(&mp3_data[..])).samples().ok_or("Invalid MP3 file")?;

    let mut source = signal::from_iter(samples.map(|sample| [sample.0, sample.1]));
    let interp = Linear::from_source(&mut source);
    let mut out_iter =
        source.from_hz_to_hz(interp, header.sample_rate.hz().into(), format.sample_rate.0.into());

    event_loop.run(move |_, data| match data {
        cpal::StreamData::Output {
            buffer: cpal::UnknownTypeOutputBuffer::F32(mut buffer),
        } => {
            for sample in buffer.chunks_mut(format.channels as usize) {
                if out_iter.is_exhausted() {
                    std::process::exit(0);
                }

                let samples = out_iter.next();
                sample[0] = samples[0];
                sample[1] = samples[1];
            }
        }
        cpal::StreamData::Output {
            buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer),
        } => {
            for sample in buffer.chunks_mut(format.channels as usize) {
                if out_iter.is_exhausted() {
                    std::process::exit(0);
                }

                let samples: [i16; 2] = out_iter.next().map(Sample::to_sample);
                sample[0] = samples[0];
                sample[1] = samples[1];
            }
        }
        cpal::StreamData::Output {
            buffer: cpal::UnknownTypeOutputBuffer::U16(mut buffer),
        } => {
            for sample in buffer.chunks_mut(format.channels as usize) {
                if out_iter.is_exhausted() {
                    std::process::exit(0);
                }

                let samples: [u16; 2] = out_iter.next().map(Sample::to_sample);
                sample[0] = samples[0];
                sample[1] = samples[1];
            }
        }
        _ => (),
    });
}
