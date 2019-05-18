use byteorder::ReadBytesExt;
use std::io::Read;

// (l, s)
const SCALE_FACTOR_BAND_INDICES: [([u32; 23], [u32; 14]); 3] = [
    (
        [
            0, 4, 8, 12, 16, 20, 24, 30, 36, 44, 52, 62, 74, 90, 110, 134, 162, 196, 238, 288, 342,
            418, 576,
        ],
        [0, 4, 8, 12, 16, 22, 30, 40, 52, 66, 84, 106, 136, 192],
    ),
    (
        [
            0, 4, 8, 12, 16, 20, 24, 30, 36, 42, 50, 60, 72, 88, 106, 128, 156, 190, 230, 276, 330,
            384, 576,
        ],
        [0, 4, 8, 12, 16, 22, 28, 38, 50, 64, 80, 100, 126, 192],
    ),
    (
        [
            0, 4, 8, 12, 16, 20, 24, 30, 36, 44, 54, 66, 82, 102, 126, 156, 194, 240, 296, 364,
            448, 550, 576,
        ],
        [0, 4, 8, 12, 16, 22, 30, 42, 58, 78, 104, 138, 180, 192],
    ),
];

const SCALE_FACTOR_SIZES: [[u32; 2]; 16] = [
    [0, 0],
    [0, 1],
    [0, 2],
    [0, 3],
    [3, 0],
    [1, 1],
    [1, 2],
    [1, 3],
    [2, 1],
    [2, 2],
    [2, 3],
    [3, 1],
    [3, 2],
    [3, 3],
    [4, 2],
    [4, 3],
];

fn read_mp3_frame<R: Read>(mut data: R) -> Result<Mp3Frame, Box<std::error::Error>> {
    if data.read_u8()? != 0xff {
        return Err("Not an MP3 frame")?;
    }

    let byte = data.read_u8()?;
    if byte & 0b1110_0000 != 0b1110_0000 {
        return Err("Not an MP3 frame")?;
    }

    let version = match byte & 0b0001_1000 {
        0b00_000 => MpegVersion::Mpeg25,
        0b01_000 => return Err("Reserved MPEG version".into()),
        0b10_000 => MpegVersion::Mpeg2,
        0b11_000 => MpegVersion::Mpeg1,
        _ => unreachable!(),
    };

    let layer = match byte & 0b110 {
        0b000 => return Err("Reserved MPEG layer".into()),
        0b010 => MpegLayer::Layer3,
        0b100 => MpegLayer::Layer2,
        0b110 => MpegLayer::Layer1,
        _ => unreachable!(),
    };

    let crc = byte & 1 != 0;

    let mut bytes = [0u8; 2];
    data.read_exact(&mut bytes)?;

    let bitrate = match (bytes[0] & 0b1111_0000, version, layer) {
        (0b0000_0000, _, _) => Bitrate::Free,
        // (0b0001_0000, MpegVersion::Mpeg1, MpegLayer::Layer2) => Bitrate::Kbps(8),
        // (0b0001_0000, MpegVersion::Mpeg1, MpegLayer::Layer2) => Bitrate::Kbps(8),
        // (0b0001_0000, MpegVersion::Mpeg1, MpegLayer::Layer3) => Bitrate::Kbps(8),
        // (0b0001_0000, _) => Bitrate::Kbps(32),
        _ => panic!(),
    };

    let sample_rate = match (bytes[0] & 0b0000_1100, version) {
        (0b00_00, MpegVersion::Mpeg1) => 44100,
        (0b00_00, MpegVersion::Mpeg2) => 22050,
        (0b00_00, MpegVersion::Mpeg25) => 11025,
        (0b01_00, MpegVersion::Mpeg1) => 48000,
        (0b01_00, MpegVersion::Mpeg2) => 24000,
        (0b01_00, MpegVersion::Mpeg25) => 12000,
        (0b10_00, MpegVersion::Mpeg1) => 32000,
        (0b10_00, MpegVersion::Mpeg2) => 16000,
        (0b10_00, MpegVersion::Mpeg25) => 8000,
        _ => return Err("Invalid sample rate".into()),
    };

    let padding = bytes[0] & 0b10 != 0;

    let channels = match bytes[1] & 0b11_000000 {
        0b00_000000 => Channels::Stereo,
        0b01_000000 => Channels::JointStereo,
        0b10_000000 => Channels::DualMono,
        0b11_000000 => Channels::Mono,
        _ => unreachable!(),
    };

    // let extension = match bytes[1] & 0b00_11_0000 {
    //     //0b00_0000 =>
    // };

    let copyright = bytes[1] & 0b1000 != 0;
    let original = bytes[1] & 0b100 != 0;
    let emphasis = match bytes[1] & 0b11 {
        0b00 => Emphasis::None,
        0b01 => Emphasis::FiftyFifteen,
        0b10 => return Err("Invalid emphasis".into()),
        0b11 => Emphasis::CcitJ17,
        _ => unreachable!(),
    };

    Ok(Mp3Frame {
        version,
        layer,
        crc,
        bitrate,
        sample_rate,
        padding,
        channels,
        copyright,
        original,
        emphasis,
    })
}

pub struct Mp3Frame {
    version: MpegVersion,
    layer: MpegLayer,
    crc: bool,
    bitrate: Bitrate,
    sample_rate: u16,
    padding: bool,
    channels: Channels,
    // Mode ext
    copyright: bool,
    original: bool,
    emphasis: Emphasis,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[allow(clippy::enum_variant_names)]
enum MpegVersion {
    Mpeg1,
    Mpeg2,
    Mpeg25,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[allow(clippy::enum_variant_names)]
enum MpegLayer {
    Layer1,
    Layer2,
    Layer3,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Channels {
    Mono,
    DualMono,
    Stereo,
    JointStereo,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Bitrate {
    Free,
    Kbps(u32),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Emphasis {
    None,
    FiftyFifteen,
    CcitJ17,
}
