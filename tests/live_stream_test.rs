use puremp3::Mp3Decoder;

const FRAME_DATA_SIZE: usize = 413;
const MAIN_DATA_BEGIN: u16 = 300;

/// Builds a mono MPEG-1 Layer III frame at 128 kbps / 44100 Hz whose side
/// info references reservoir bytes from before the start of the stream.
///
/// Every side info field except main_data_begin is left at zero, so the
/// frame needs no real encoded audio data to reach the code that copies the
/// bit reservoir forward.
fn frame_with_main_data_begin(main_data_begin: u16) -> Vec<u8> {
    let mut frame = vec![0u8; 4 + FRAME_DATA_SIZE];

    frame[0] = 0xff;
    frame[1] = 0xfb; // MPEG-1 Layer III, no CRC.
    frame[2] = 0x90; // 128 kbps, 44100 Hz, no padding.
    frame[3] = 0xc0; // Mono.

    // main_data_begin is the first 9 bits of the side info.
    frame[4] = (main_data_begin >> 1) as u8;
    frame[5] = ((main_data_begin & 1) << 7) as u8;

    frame
}

#[test]
fn first_frame_with_reservoir_reference_does_not_panic() {
    let data = frame_with_main_data_begin(MAIN_DATA_BEGIN);
    let mut decoder = Mp3Decoder::new(&data[..]);

    // Before the fix, the empty frame buffer plus a nonzero
    // main_data_begin underflowed and panicked here.
    let frame = decoder
        .next_frame()
        .expect("first frame should decode with its missing reservoir filled with zeros");
    assert_eq!(frame.num_samples, 1152);
    assert!(frame
        .samples
        .iter()
        .flatten()
        .all(|sample| sample.is_finite()));
}
