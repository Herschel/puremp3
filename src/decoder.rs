use crate::error::{Error, Mp3Error};
use crate::tables::{LFS_INTENSITY_STEREO_TABLE, LFS_TABLE, SCALE_FACTOR_SIZES};
use bitstream_io::{BigEndian, BitReader};
use byteorder::ReadBytesExt;
use std::io::Read;

pub const MAX_CHANNELS: usize = 2;
pub const MAX_GRANULES: usize = 2;

pub struct Decoder {
    frame_buffer: [u8; 4096],
    frame_buffer_len: usize,
    store: [[[f32; 18]; 32]; 2],
    sbs_v_vec: [[f32; 1024]; 2],
}

impl Decoder {
    pub fn new() -> Self {
        Self {
            frame_buffer: [0; 4096],
            frame_buffer_len: 0,
            store: [[[0f32; 18]; 32]; 2],
            sbs_v_vec: [[0f32; 1024]; 2],
        }
    }
}

pub fn read_frame_header<R: Read>(mut data: R) -> Result<FrameHeader, Error> {
    if data.read_u8()? != 0xff {
        return Err(Error::Mp3Error(Mp3Error::InvalidData(
            "Frame sync not found",
        )));
    }

    let byte = data.read_u8()?;
    if byte & 0b1110_0000 != 0b1110_0000 {
        return Err(Error::Mp3Error(Mp3Error::InvalidData(
            "Frame sync not found",
        )));
    }

    let version = match byte & 0b0001_1000 {
        0b00_000 => MpegVersion::Mpeg2_5,
        0b01_000 => {
            return Err(Error::Mp3Error(Mp3Error::InvalidData(
                "Invalid MPEG version",
            )))
        }
        0b10_000 => MpegVersion::Mpeg2,
        0b11_000 => MpegVersion::Mpeg1,
        _ => unreachable!(),
    };

    let layer = match byte & 0b110 {
        0b000 => return Err(Error::Mp3Error(Mp3Error::InvalidData("Invalid MPEG layer"))),
        0b010 => MpegLayer::Layer3,
        0b100 => MpegLayer::Layer2,
        0b110 => MpegLayer::Layer1,
        _ => unreachable!(),
    };
    if layer != MpegLayer::Layer3 {
        return Err(Error::Mp3Error(Mp3Error::Unsupported(
            "Only MPEG Layer III is supported",
        )));
    }

    // CRC is ignored for now.
    let crc = byte & 1 == 0;

    let mut bytes = [0u8; 2];
    data.read_exact(&mut bytes)?;

    let is_version2 = version == MpegVersion::Mpeg2 || version == MpegVersion::Mpeg2_5;
    let bitrate = match (bytes[0] & 0b1111_0000, is_version2) {
        (0b0001_0000, false) => BitRate::Kbps32,
        (0b0010_0000, false) => BitRate::Kbps40,
        (0b0011_0000, false) => BitRate::Kbps48,
        (0b0100_0000, false) => BitRate::Kbps56,
        (0b0101_0000, false) => BitRate::Kbps64,
        (0b0110_0000, false) => BitRate::Kbps80,
        (0b0111_0000, false) => BitRate::Kbps96,
        (0b1000_0000, false) => BitRate::Kbps112,
        (0b1001_0000, false) => BitRate::Kbps128,
        (0b1010_0000, false) => BitRate::Kbps160,
        (0b1011_0000, false) => BitRate::Kbps192,
        (0b1100_0000, false) => BitRate::Kbps224,
        (0b1101_0000, false) => BitRate::Kbps256,
        (0b1110_0000, false) => BitRate::Kbps320,

        (0b0001_0000, true) => BitRate::Kbps8,
        (0b0010_0000, true) => BitRate::Kbps16,
        (0b0011_0000, true) => BitRate::Kbps24,
        (0b0100_0000, true) => BitRate::Kbps32,
        (0b0101_0000, true) => BitRate::Kbps40,
        (0b0110_0000, true) => BitRate::Kbps48,
        (0b0111_0000, true) => BitRate::Kbps56,
        (0b1000_0000, true) => BitRate::Kbps64,
        (0b1001_0000, true) => BitRate::Kbps80,
        (0b1010_0000, true) => BitRate::Kbps96,
        (0b1011_0000, true) => BitRate::Kbps112,
        (0b1100_0000, true) => BitRate::Kbps128,
        (0b1101_0000, true) => BitRate::Kbps144,
        (0b1110_0000, true) => BitRate::Kbps160,

        (0b0000_0000, _) => {
            return Err(Error::Mp3Error(Mp3Error::Unsupported(
                "Free bitrate is unsupported",
            )))
        }
        _ => return Err(Error::Mp3Error(Mp3Error::InvalidData("Invalid bitrate"))),
    };

    let sample_rate = match (bytes[0] & 0b0000_1100, version) {
        (0b00_00, MpegVersion::Mpeg1) => SampleRate::Hz44100,
        (0b00_00, MpegVersion::Mpeg2) => SampleRate::Hz22050,
        (0b00_00, MpegVersion::Mpeg2_5) => SampleRate::Hz11025,
        (0b01_00, MpegVersion::Mpeg1) => SampleRate::Hz48000,
        (0b01_00, MpegVersion::Mpeg2) => SampleRate::Hz24000,
        (0b01_00, MpegVersion::Mpeg2_5) => SampleRate::Hz12000,
        (0b10_00, MpegVersion::Mpeg1) => SampleRate::Hz32000,
        (0b10_00, MpegVersion::Mpeg2) => SampleRate::Hz16000,
        (0b10_00, MpegVersion::Mpeg2_5) => SampleRate::Hz8000,
        _ => return Err(Error::Mp3Error(Mp3Error::InvalidData("Invalid bitrate"))),
    };
    let sample_rate_table = ((bytes[0] & 0b0000_1100) >> 2) as usize
        + match version {
            MpegVersion::Mpeg1 => 0,
            MpegVersion::Mpeg2 => 3,
            MpegVersion::Mpeg2_5 => 6,
        };

    let padding = bytes[0] & 0b10 != 0;

    let channels = match bytes[1] & 0b11_000000 {
        0b00_000000 => Channels::Stereo,
        0b01_000000 => Channels::JointStereo {
            mid_side_stereo: bytes[1] & 0b0010_0000 != 0,
            intensity_stereo: bytes[1] & 0b0001_0000 != 0,
        },
        0b10_000000 => Channels::DualMono,
        0b11_000000 => Channels::Mono,
        _ => unreachable!(),
    };

    let copyright = bytes[1] & 0b1000 != 0;
    let original = bytes[1] & 0b100 != 0;
    let emphasis = match bytes[1] & 0b11 {
        0b00 => Emphasis::None,
        0b01 => Emphasis::FiftyFifteen,
        0b10 => return Err(Error::Mp3Error(Mp3Error::InvalidData("Invalid emphasis"))),
        0b11 => Emphasis::CcitJ17,
        _ => unreachable!(),
    };

    if crc {
        // Skip CRC for now.
        data.read_u8()?;
        data.read_u8()?;
    }

    let bits_per_sample = match version {
        MpegVersion::Mpeg1 => 144,
        MpegVersion::Mpeg2 => 72,
        MpegVersion::Mpeg2_5 => 72,
    };
    let data_size = (bits_per_sample * bitrate.bps() / sample_rate.hz()
        + if padding { 1 } else { 0 }
        - if crc { 2 } else { 0 }
        - 4) as usize;

    // Skip framesize?
    // Skip ancillary data...?

    Ok(FrameHeader {
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

        sample_rate_table,
        data_size,
    })
}

#[derive(Debug)]
pub struct FrameHeader {
    pub version: MpegVersion,
    pub layer: MpegLayer,
    pub crc: bool,
    pub bitrate: BitRate,
    pub sample_rate: SampleRate,
    pub padding: bool,
    pub channels: Channels,
    pub copyright: bool,
    pub original: bool,
    pub emphasis: Emphasis,
    pub sample_rate_table: usize,
    pub data_size: usize,
}

impl FrameHeader {
    fn side_data_len(&self) -> usize {
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

    fn num_granules(&self) -> usize {
        if self.version == MpegVersion::Mpeg1 {
            2
        } else {
            1
        }
    }

    fn is_intensity_stereo(&self) -> bool {
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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum MpegVersion {
    Mpeg1,
    Mpeg2,
    Mpeg2_5,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum MpegLayer {
    Layer1,
    Layer2,
    Layer3,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Channels {
    Mono,
    DualMono,
    Stereo,
    JointStereo {
        intensity_stereo: bool,
        mid_side_stereo: bool,
    },
}

impl Channels {
    pub fn num_channels(self) -> usize {
        match self {
            Channels::Mono => 1,
            _ => 2,
        }
    }
}

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
    fn bps(self) -> u32 {
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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Emphasis {
    None,
    FiftyFifteen,
    CcitJ17,
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

// TODO(Herschel): Better name for this

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

fn read_side_info<R: Read>(mut data: R, header: &FrameHeader) -> Result<SideInfo, Error> {
    let mut info: SideInfo = Default::default();
    let mut bytes = [0u8; 32];
    let size = header.side_data_len();
    data.read_exact(&mut bytes[..size])?;

    let mut reader = BitReader::endian(&bytes[..], BigEndian);

    if header.version == MpegVersion::Mpeg1 {
        info.main_data_begin = reader.read(9)?;

        // Skip private bits.
        if header.channels == Channels::Mono {
            reader.skip(5)?;
        } else {
            reader.skip(3)?;
        }

        for scfsi in &mut info.scfsi[..header.channels.num_channels()] {
            for band in scfsi.iter_mut() {
                *band = reader.read_bit()?;
            }
        }
    } else {
        info.main_data_begin = reader.read(8)?;

        // Skip private bits.
        if header.channels == Channels::Mono {
            reader.skip(1)?;
        } else {
            reader.skip(2)?;
        }
    }

    for granule in &mut info.granules[..header.num_granules()] {
        *granule = read_granule_side_info(&header, &mut reader)?;
    }

    Ok(info)
}

fn read_granule_side_info<R: Read>(
    header: &FrameHeader,
    reader: &mut BitReader<R, BigEndian>,
) -> Result<GranuleSideInfo, Error> {
    let mut info: GranuleSideInfo = Default::default();
    for channel_side_info in &mut info.channels[0..header.channels.num_channels()] {
        *channel_side_info = read_granule_channel_side_info(header, reader)?;
    }
    Ok(info)
}

fn read_granule_channel_side_info<R: Read>(
    header: &FrameHeader,
    reader: &mut BitReader<R, BigEndian>,
) -> Result<GranuleChannelSideInfo, Error> {
    let mut info: GranuleChannelSideInfo = Default::default();

    info.part2_3_length = reader.read(12)?;
    info.big_values = reader.read(9)?;
    if info.big_values > 288 {
        return Err(Error::Mp3Error(Mp3Error::InvalidData("big_values > 288")));
    }
    info.global_gain = reader.read(8)?;
    let scalefac_compress_len = if header.version == MpegVersion::Mpeg1 {
        4
    } else {
        9
    };
    info.scalefac_compress = reader.read(scalefac_compress_len)?;

    let window_switching = reader.read_bit()?;
    if window_switching {
        let block_type_id = reader.read::<u8>(2)?;
        let mixed_block = reader.read_bit()?;
        for region in &mut info.table_select[..2] {
            *region = reader.read(5)?;
        }

        let mut subblock_gain = [0f32; 3];
        for gain in &mut subblock_gain {
            *gain = reader.read::<u8>(3)?.into();
        }
        info.subblock_gain = subblock_gain;

        info.block_type = match block_type_id {
            0b00 => {
                // Block type 00 is only if window switching is off
                return Err(Error::Mp3Error(Mp3Error::InvalidData(
                    "Forbidden block type",
                )));
            }
            0b01 => BlockType::Start,
            0b10 => {
                if mixed_block {
                    BlockType::Mixed
                } else {
                    BlockType::Short
                }
            }
            0b11 => BlockType::End,
            _ => unreachable!(),
        };

        // Mixed blocks are always marked as short.
        assert!(!mixed_block || info.block_type == BlockType::Short);

        info.region0_count = if info.block_type == BlockType::Short {
            8
        } else {
            7
        };
        info.region1_count = 20 - info.region0_count;
    } else {
        info.block_type = BlockType::Long;

        for region in &mut info.table_select {
            *region = reader.read(5)?;
        }

        info.region0_count = reader.read(4)?;
        info.region1_count = reader.read(3)?;
    }

    info.preflag = if header.version == MpegVersion::Mpeg1 {
        reader.read_bit()?
    } else {
        info.scalefac_compress >= 500
    };

    info.scalefac_scale = reader.read_bit()?; // .5f * (1f + frame.ReadBits(1));
    info.count1table_select = reader.read_bit()?;

    Ok(info)
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

fn read_logical_frame_data<'a, R: Read>(
    decoder: &'a mut Decoder,
    mut reader: R,
    header: &FrameHeader,
    side_info: &SideInfo,
) -> Result<&'a [u8], Error> {
    let side_info_size = header.side_data_len();
    let main_data_size = header.data_size - side_info_size;

    // Copy main_data_begin bytes from the previous frame(s).
    let main_data_begin = side_info.main_data_begin as usize;
    let prev_start = decoder.frame_buffer_len - main_data_begin;
    for i in 0..main_data_begin {
        decoder.frame_buffer[i] = decoder.frame_buffer[prev_start + i];
    }
    decoder.frame_buffer_len = main_data_begin + main_data_size;
    reader.read_exact(&mut decoder.frame_buffer[main_data_begin..decoder.frame_buffer_len])?;

    Ok(&decoder.frame_buffer[0..decoder.frame_buffer_len])
}

fn read_main_data<R: Read>(
    reader: &mut BitReader<R, BigEndian>,
    header: &FrameHeader,
    side_info: &SideInfo,
) -> Result<MainData, Error> {
    let mut data: MainData = Default::default();

    for g in 0..header.num_granules() {
        for c in 0..header.channels.num_channels() {
            let bits_read = if header.version == MpegVersion::Mpeg1 {
                read_scale_factors(reader, g, c, &side_info, &mut data)?
            } else {
                read_lfs_scale_factors(
                    reader,
                    c == 1 && header.is_intensity_stereo(),
                    &side_info.granules[g].channels[c],
                    &mut data.granules[g].channels[c],
                )?
            };

            let huffman_len =
                u32::from(side_info.granules[g].channels[c].part2_3_length) - bits_read;
            data.granules[g].channels[c].count1 = crate::huffman::read_huffman(
                reader,
                header,
                &side_info.granules[g].channels[c],
                huffman_len,
                &mut data.granules[g].channels[c].samples,
            )?;
        }
    }

    // TODO(Herschel): Skip ancillary data.
    Ok(data)
}

pub fn read_scale_factors<R: Read>(
    reader: &mut BitReader<R, BigEndian>,
    granule: usize,
    channel: usize,
    side_info: &SideInfo,
    main_data: &mut MainData,
) -> Result<u32, Error> {
    let mut bits_read = 0;

    let block_type = side_info.granules[granule].channels[channel].block_type;
    let scalefac_compress =
        side_info.granules[granule].channels[channel].scalefac_compress as usize;
    let (scale_len1, scale_len2) = SCALE_FACTOR_SIZES[scalefac_compress];

    if block_type == BlockType::Short || block_type == BlockType::Mixed {
        let channel_info = &side_info.granules[granule].channels[channel];
        let channel_data = &mut main_data.granules[granule].channels[channel];
        if scale_len1 > 0 {
            if channel_info.block_type == BlockType::Mixed {
                for sfb in &mut channel_data.scalefac_l[..8] {
                    *sfb = reader.read(scale_len1 as u32)?;
                    bits_read += scale_len1;
                }
            }

            for sfb in &mut channel_data.scalefac_s[..6] {
                for window in sfb.iter_mut() {
                    *window = reader.read(scale_len1 as u32)?;
                    bits_read += scale_len1;
                }
            }
        }

        if scale_len2 > 0 {
            for sfb in &mut channel_data.scalefac_s[6..12] {
                for window in sfb.iter_mut() {
                    *window = reader.read(scale_len2 as u32)?;
                    bits_read += scale_len2;
                }
            }
        }
    } else {
        // Normal window.
        let slices = [(0usize, 6usize), (6, 11), (11, 16), (16, 21)];
        for (i, (start, end)) in slices.iter().enumerate() {
            let len = if i < 2 { scale_len1 } else { scale_len2 } as u32;
            if len > 0 {
                if granule == 0 || !side_info.scfsi[channel][i] {
                    for sfb in
                        &mut main_data.granules[granule].channels[channel].scalefac_l[*start..*end]
                    {
                        *sfb = reader.read(len)?;
                        bits_read += len;
                    }
                // TODO(Herschel): Is there a cleaner way to do this?
                // Granule can copy from previous granule. I would like to write this fn without
                // using array accesses.
                } else if granule == 1 && side_info.scfsi[channel][i] {
                    let (granule0, granules) = main_data.granules.split_first_mut().unwrap();
                    granule0.channels[channel].scalefac_l[*start..*end]
                        .copy_from_slice(&granules[0].channels[channel].scalefac_l[*start..*end]);
                }
            }
        }
    }

    Ok(bits_read)
}

fn read_lfs_scale_factors<R: Read>(
    reader: &mut BitReader<R, BigEndian>,
    intensity_stereo_channel: bool,
    channel_info: &GranuleChannelSideInfo,
    channel_data: &mut MainDataChannel,
) -> Result<u32, Error> {
    let mut bits_read = 0;

    let lfs_table = if intensity_stereo_channel {
        &LFS_INTENSITY_STEREO_TABLE
    } else {
        &LFS_TABLE
    };
    let lfs_table = match channel_info.block_type {
        BlockType::Short => &lfs_table[1],
        BlockType::Mixed => &lfs_table[2],
        _ => &lfs_table[0],
    };

    let (scale_lens, lfs_table) = if intensity_stereo_channel {
        let sfc = u32::from(channel_info.scalefac_compress / 2);
        match sfc {
            0...179 => ([sfc / 36, (sfc % 36) / 6, sfc % 6, 0], &lfs_table[0]),
            180...243 => (
                [
                    ((sfc - 180) % 64) / 16,
                    ((sfc - 180) % 16) / 4,
                    (sfc - 180) % 4,
                    0,
                ],
                &lfs_table[1],
            ),
            244...255 => ([(sfc - 244) / 3, (sfc - 244) % 3, 0, 0], &lfs_table[2]),
            _ => unreachable!(),
        }
    } else {
        let sfc = u32::from(channel_info.scalefac_compress);
        match sfc {
            0...399 => (
                [sfc / 80, (sfc / 16) % 5, (sfc % 16) / 4, sfc & 3],
                &lfs_table[0],
            ),
            400...499 => (
                [(sfc - 400) / 20, ((sfc - 400) / 4) % 5, (sfc - 400) % 4, 0],
                &lfs_table[1],
            ),
            500...512 => ([(sfc - 500) / 3, (sfc - 500) % 3, 0, 0], &lfs_table[2]),
            _ => unreachable!(),
        }
    };

    // TODO(Herschel): We could avoid using this intermediate buffer.
    // Write an iterator for reading scalefacs and/or write an iterator
    // through scalefac_s/l for the block type.
    let mut scalefacs = [0u8; 54];
    let mut i = 0;
    for (&len, &num_blocks) in scale_lens[..].iter().zip(lfs_table.iter()) {
        assert!(len <= 8);
        if len > 0 {
            for _ in 0..num_blocks {
                scalefacs[i] = reader.read(len)?;
                bits_read += len;
                i += 1;
            }
        } else {
            i += num_blocks;
        }
    }

    i = 0;
    if channel_info.block_type == BlockType::Short || channel_info.block_type == BlockType::Mixed {
        let short_start = if channel_info.block_type == BlockType::Mixed {
            for sfb in 0..8 {
                channel_data.scalefac_l[sfb] = scalefacs[i];
                i += 1;
            }
            3
        } else {
            0
        };

        for sfb in short_start..12 {
            for window in 0..3 {
                channel_data.scalefac_s[sfb][window] = scalefacs[i];
                i += 1;
            }
        }
    } else {
        for sfb in 0..21 {
            channel_data.scalefac_l[sfb] = scalefacs[i];
            i += 1;
        }
    }

    Ok(bits_read)
}

pub fn process_frame<R: Read>(
    decoder: &mut Decoder,
    mut reader: R,
    header: &FrameHeader,
) -> Result<(usize, [[f32; 1152]; 2]), Error> {
    let side_info = read_side_info(&mut reader, header)?;
    let data_buffer = read_logical_frame_data(decoder, &mut reader, header, &side_info)?;

    let mut reader = BitReader::endian(data_buffer, BigEndian);
    let mut main_data = read_main_data(&mut reader, header, &side_info)?;

    let mut out_samples = [[0f32; 1152]; 2];
    let num_samples = decode_frame(
        decoder,
        header,
        &side_info,
        &mut main_data,
        &mut out_samples,
    )?;

    Ok((num_samples, out_samples))
}

fn decode_frame(
    decoder: &mut Decoder,
    header: &FrameHeader,
    side_info: &SideInfo,
    main_data: &mut MainData,
    out_samples: &mut [[f32; 1152]; 2],
) -> Result<usize, Error> {
    use crate::{requantize, stereo, synthesis};

    if header.channels == Channels::Mono {
        for gr in 0..header.num_granules() {
            let side_info = &side_info.granules[gr].channels[0];
            let main_data = &mut main_data.granules[gr].channels[0];

            requantize::requantize(header, side_info, main_data);
            requantize::reorder(header, side_info, main_data);
            synthesis::antialias(side_info, &mut main_data.samples);
            synthesis::hybrid_synthesis(
                side_info.block_type,
                &mut decoder.store[0],
                &mut main_data.samples,
            );
            synthesis::frequency_inversion(&mut main_data.samples);
            synthesis::subband_synthesis(
                &main_data.samples,
                &mut decoder.sbs_v_vec[0],
                &mut out_samples[0][gr * 576..(gr + 1) * 576],
            );
        }

        out_samples[1] = out_samples[0];
    } else {
        for gr in 0..header.num_granules() {
            for ch in 0..MAX_CHANNELS {
                let side_info = &side_info.granules[gr].channels[ch];
                let main_data = &mut main_data.granules[gr].channels[ch];

                requantize::requantize(header, side_info, main_data);
                requantize::reorder(header, side_info, main_data);
            }

            if let Channels::JointStereo {
                intensity_stereo,
                mid_side_stereo,
            } = header.channels
            {
                stereo::stereo(
                    header,
                    &side_info.granules[gr],
                    intensity_stereo,
                    mid_side_stereo,
                    &mut main_data.granules[gr],
                );
            }

            for (ch, out_channel) in out_samples.iter_mut().enumerate() {
                let side_info = &side_info.granules[gr].channels[ch];
                let main_data = &mut main_data.granules[gr].channels[ch];

                synthesis::antialias(side_info, &mut main_data.samples);
                synthesis::hybrid_synthesis(
                    side_info.block_type,
                    &mut decoder.store[ch],
                    &mut main_data.samples,
                );
                synthesis::frequency_inversion(&mut main_data.samples);
                synthesis::subband_synthesis(
                    &main_data.samples,
                    &mut decoder.sbs_v_vec[ch],
                    &mut out_channel[gr * 576..(gr + 1) * 576],
                );
            }
        }
    }
    Ok(header.num_granules() * 576)
}
