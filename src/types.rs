//! Types and data structures used by the MP3 decoder.

/// The maximum number of channels supported in an MP3.
pub const MAX_CHANNELS: usize = 2;

/// The maximum number of granules in an MP3 frame.AsMut
///
/// Depends on the MPEG version.
pub(crate) const MAX_GRANULES: usize = 2;

/// Header of an MP3 frame.
///
/// Contains info about the format of the audio samples.
#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// The MPEG standard used in encoding this frame.
    pub version: MpegVersion,

    /// The MPEG layer of the frame.
    ///
    /// Currently only MPEG Layer III is supported.
    pub layer: MpegLayer,

    /// Whether the frame contains a CRC checksum.
    pub crc: bool,

    /// The bitrate of this frame.
    pub bitrate: BitRate,

    /// The sample rate of this frame.
    pub sample_rate: SampleRate,

    /// Whether the frame has an extra padding bit.
    pub padding: bool,

    /// The channel mode of this frame.
    pub channels: Channels,

    /// Whether this frame is under copyright.
    pub copyright: bool,

    /// Whether this frame contains original data or a copy.
    pub original: bool,

    /// The emphasis of this frame.
    pub emphasis: Emphasis,

    pub(crate) sample_rate_table: usize,
    pub(crate) data_size: usize,
}

impl FrameHeader {
    pub(crate) fn side_data_len(&self) -> usize {
        match self.layer {
            MpegLayer::Layer3 => {
                if self.channels == Channels::Mono && self.version != MpegVersion::Mpeg1 {
                    9
                } else if self.channels != Channels::Mono && self.version == MpegVersion::Mpeg1 {
                    32
                } else {
                    17
                }
            }
            _ => unimplemented!(),
        }
    }

    pub(crate) fn num_granules(&self) -> usize {
        if self.version == MpegVersion::Mpeg1 {
            2
        } else {
            1
        }
    }

    pub(crate) fn is_intensity_stereo(&self) -> bool {
        if let Channels::JointStereo {
            intensity_stereo: true,
            ..
        } = self.channels
        {
            true
        } else {
            false
        }
    }
}

/// The version of the MPEG standard used in encoding audio.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum MpegVersion {
    /// MPEG-1 (ISO/IEC 11172-3)
    Mpeg1,

    /// MPEG-2 (ISO/IEC 13818-3)
    Mpeg2,

    /// MPEG-2.5
    Mpeg2_5,
}

/// The MPEG Layer used in encoding audio.
///
/// Higher layers provide better compression, but are more complex.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum MpegLayer {
    /// MPEG Layer I
    Layer1,

    /// MPEG Layer II
    Layer2,

    /// MPEG Layer III
    Layer3,
}

/// The channel mode
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Channels {
    /// One audio channel.
    Mono,

    /// Two unrelated audio channels (e.g. for different languages).
    DualMono,

    /// Stereo.
    Stereo,

    /// Joint stereo. Improves compression by utilizing the correlation
    /// in stereo channels.
    JointStereo {
        intensity_stereo: bool,
        mid_side_stereo: bool,
    },
}

impl Channels {
    /// The number of audio channels.
    pub fn num_channels(self) -> usize {
        match self {
            Channels::Mono => 1,
            _ => 2,
        }
    }
}

/// The bit rate of an MP3 stream.
///
/// MP3 supports specific bitrates, depending on the MPEG version and layer.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum BitRate {
    Kbps8,
    Kbps16,
    Kbps24,
    Kbps32,
    Kbps40,
    Kbps48,
    Kbps56,
    Kbps64,
    Kbps80,
    Kbps96,
    Kbps112,
    Kbps128,
    Kbps144,
    Kbps160,
    Kbps192,
    Kbps224,
    Kbps256,
    Kbps320,
}

impl BitRate {
    /// Returns the bit rate in bits per second as a `u32`.
    pub fn bps(self) -> u32 {
        match self {
            BitRate::Kbps8 => 8_000,
            BitRate::Kbps16 => 16_000,
            BitRate::Kbps24 => 24_000,
            BitRate::Kbps32 => 32_000,
            BitRate::Kbps40 => 40_000,
            BitRate::Kbps48 => 48_000,
            BitRate::Kbps56 => 56_000,
            BitRate::Kbps64 => 64_000,
            BitRate::Kbps80 => 80_000,
            BitRate::Kbps96 => 96_000,
            BitRate::Kbps112 => 112_000,
            BitRate::Kbps128 => 128_000,
            BitRate::Kbps144 => 144_000,
            BitRate::Kbps160 => 160_000,
            BitRate::Kbps192 => 192_000,
            BitRate::Kbps224 => 224_000,
            BitRate::Kbps256 => 256_000,
            BitRate::Kbps320 => 320_000,
        }
    }
}

/// The sample rate of an MP3 stream.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SampleRate {
    Hz8000,
    Hz11025,
    Hz12000,
    Hz16000,
    Hz22050,
    Hz24000,
    Hz32000,
    Hz44100,
    Hz48000,
}

impl SampleRate {
    /// Returns the sample rate in hertz as a `u32`.
    pub fn hz(self) -> u32 {
        match self {
            SampleRate::Hz8000 => 8_000,
            SampleRate::Hz11025 => 11_025,
            SampleRate::Hz12000 => 12_000,
            SampleRate::Hz16000 => 16_000,
            SampleRate::Hz22050 => 22_050,
            SampleRate::Hz24000 => 24_000,
            SampleRate::Hz32000 => 32_000,
            SampleRate::Hz44100 => 44_100,
            SampleRate::Hz48000 => 48_000,
        }
    }
}

/// Emphasis used in encoding an MP3 audio stream.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Emphasis {
    None,
    FiftyFifteen,
    CcitJ17,
}

// Internal types
pub struct DecoderState {
    pub frame_buffer: [u8; 4096],
    pub frame_buffer_len: usize,
    pub store: [[[f32; 18]; 32]; 2],
    pub sbs_v_vec: [[f32; 1024]; 2],
}

impl DecoderState {
    pub fn new() -> Self {
        DecoderState {
            frame_buffer: [0; 4096],
            frame_buffer_len: 0,
            store: [[[0f32; 18]; 32]; 2],
            sbs_v_vec: [[0f32; 1024]; 2],
        }
    }
}

#[derive(Debug, Default)]
pub struct SideInfo {
    pub main_data_begin: u16,
    pub scfsi: [[bool; 4]; 2], // Scale Factor Selection Information
    pub granules: [GranuleSideInfo; 2],
}

#[derive(Debug, Default)]
pub struct GranuleSideInfo {
    pub channels: [GranuleChannelSideInfo; 2],
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BlockType {
    Long,
    Short,
    Mixed,
    Start,
    End,
}

impl Default for BlockType {
    fn default() -> BlockType {
        BlockType::Long
    }
}

#[derive(Debug, Default)]
pub struct GranuleChannelSideInfo {
    pub part2_3_length: u16,
    pub big_values: u16,
    pub global_gain: u8,
    pub scalefac_compress: u16,
    pub block_type: BlockType,
    pub mixed_block: bool,
    pub subblock_gain: [f32; 3],

    pub table_select: [u8; 3],
    pub region0_count: u8,
    pub region1_count: u8,
    pub preflag: bool,
    pub scalefac_scale: bool,
    pub count1table_select: bool,
}

#[derive(Debug, Default)]
pub struct MainData {
    pub granules: [MainDataGranule; MAX_GRANULES],
}

#[derive(Debug, Default)]
pub struct MainDataGranule {
    pub channels: [MainDataChannel; MAX_CHANNELS],
}

pub struct MainDataChannel {
    pub scalefac_l: [u8; 22],
    pub scalefac_s: [[u8; 3]; 13],
    pub count1: u32, // TODO(Herschel): What's the actual size of this?
    pub samples: [f32; 576],
}

impl Default for MainDataChannel {
    fn default() -> Self {
        Self {
            scalefac_l: Default::default(),
            scalefac_s: Default::default(),
            count1: Default::default(),
            samples: [Default::default(); 576],
        }
    }
}

impl std::fmt::Debug for MainDataChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "MainDataChannel")
    }
}

