use puremp3::Mp3Iterator;
use sample::interpolate::Linear;
use sample::{signal, Frame, Sample, Signal};

fn main() -> Result<(), Box<std::error::Error>> {
    let mut args = std::env::args();
    let input_path = args.nth(1).ok_or("Input file required")?;
    let mp3_data = std::fs::read(input_path)?;
    let mut decoder = Mp3Iterator::new(std::io::Cursor::new(&mp3_data[..]));

    let device = cpal::default_output_device().ok_or("Failed to get default output device")?;
    let format = device.default_output_format()?;
    let event_loop = cpal::EventLoop::new();
    let stream_id = event_loop.build_output_stream(&device, &format)?;
    event_loop.play_stream(stream_id.clone());

    let mut frame = decoder.next().ok_or("Invalid MP3")?;
    let mut cur_sample = 0;

    // Produce a sinusoid of maximum amplitude.
    let mut next_value = std::iter::from_fn(move || {
        if cur_sample >= 1152 {
            if let Some(new_frame) = decoder.next() {
                frame = new_frame;
                cur_sample = 0;
            } else {
                return None;
            }
        }

        let out = [frame.samples[0][cur_sample], frame.samples[1][cur_sample]];
        cur_sample += 1;
        Some(out)
    });

    let mut source = signal::from_iter(next_value);
    let interp = Linear::from_source(&mut source);
    let mut out_iter = source.from_hz_to_hz(interp, 44100.0, format.sample_rate.0 as f64);

    // let mut data_out = vec![];
    // while !out_iter.is_exhausted() {
    //     use byteorder::{LittleEndian, WriteBytesExt};
    //     use std::io::Write;
    //     let samples = out_iter.next();
    //     data_out.write_f32::<LittleEndian>(samples[0])?;
    //     data_out.write_f32::<LittleEndian>(samples[1])?;
    // }
    // std::fs::write("out.pcm", &data_out[..])?;
    // Ok(())
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
