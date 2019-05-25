use crate::types::{BlockType, GranuleChannelSideInfo};
use crate::error::Error;
use crate::tables::*;
use crate::types::FrameHeader;
use bitstream_io::{BigEndian, BitReader};
use std::io::Read;

pub fn read_huffman<R: Read>(
    reader: &mut BitReader<R, BigEndian>,
    header: &FrameHeader,
    side_info: &GranuleChannelSideInfo,
    len: u32,
    samples: &mut [f32; 576],
) -> Result<u32, Error> {
    if len == 0 {
        for sample in samples.iter_mut() {
            *sample = 0.0;
        }
        return Ok(0);
    }

    let mut bits_read = 0;
    // ? let bit_pos_end = part_2_start + side_info.part2_3_length - 1;

    let (region1_start, region2_start) =
        if side_info.block_type == BlockType::Short || side_info.block_type == BlockType::Mixed {
            (36, 576)
        } else {
            (
                SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].0
                    [side_info.region0_count as usize + 1],
                SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].0
                    [side_info.region0_count as usize + side_info.region1_count as usize + 2],
            )
        };

    // Read big_values.
    let mut is_pos: usize = 0;
    let is_len = side_info.big_values as usize * 2;
    let mut state: HuffmanState = Default::default();
    while is_pos < is_len {
        let table_num = if is_pos < region1_start as usize {
            side_info.table_select[0]
        } else if is_pos < region2_start as usize {
            side_info.table_select[1]
        } else {
            side_info.table_select[2]
        };

        let huffman_table = &HUFFMAN_TABLES[table_num as usize];
        // TODO(Herschel): Is state an inout parameter or just output?
        bits_read += huffman_decode(reader, huffman_table, &mut state)?;

        samples[is_pos] = state.x as f32;
        is_pos += 1;
        samples[is_pos] = state.y as f32;
        is_pos += 1;
    }

    // Read small values until is_pos is 576
    let table_num = if side_info.count1table_select { 33 } else { 32 };
    let huffman_table = &HUFFMAN_TABLES[table_num];
    is_pos = is_len;
    while is_pos <= 572 && bits_read < len as usize {
        bits_read += huffman_decode(reader, huffman_table, &mut state)?;
        samples[is_pos] = state.v as f32;
        is_pos += 1;
        samples[is_pos] = state.w as f32;
        is_pos += 1;
        samples[is_pos] = state.x as f32;
        is_pos += 1;
        samples[is_pos] = state.y as f32;
        is_pos += 1;
    }

    if bits_read < len as usize {
        reader.skip(len - bits_read as u32)?;
    } else if bits_read > len as usize {
        is_pos -= 4;
    }

    for sample in &mut samples[is_pos..576] {
        *sample = 0.0;
    }

    Ok(is_pos as u32)
}

#[derive(Debug, Default)]
struct HuffmanState {
    x: i32,
    y: i32,
    v: i32,
    w: i32,
}
fn huffman_decode<R: Read>(
    reader: &mut BitReader<R, BigEndian>,
    huffman_table: &HuffmanTable,
    state: &mut HuffmanState,
) -> Result<usize, Error> {
    let mut point = 0;
    let mut bits_left = 32;
    let mut bits_read = 0;
    if !huffman_table.data.is_empty() {
        loop {
            if huffman_table.data[point] & 0xff00 == 0 {
                state.x = ((huffman_table.data[point] >> 4) & 0xf).into();
                state.y = (huffman_table.data[point] & 0xf).into();
                break;
            }

            bits_read += 1;
            if reader.read_bit()? {
                while (huffman_table.data[point] & 0xff) >= 250 {
                    point += (huffman_table.data[point] & 0xff) as usize;
                }
                point += (huffman_table.data[point] & 0xff) as usize;
            } else {
                while (huffman_table.data[point] >> 8) >= 250 {
                    point += (huffman_table.data[point] >> 8) as usize;
                }
                point += (huffman_table.data[point] >> 8) as usize;
            }

            bits_left -= 1;
            if bits_left <= 0 || point >= huffman_table.data.len() {
                break;
            }
        }

        if huffman_table.quads {
            state.v = (state.y >> 3) & 1;
            state.w = (state.y >> 2) & 1;
            state.x = (state.y >> 1) & 1;
            state.y &= 1;

            if state.v > 0 {
                bits_read += 1;
                if reader.read_bit()? {
                    state.v = -state.v;
                }
            }
            if state.w > 0 {
                bits_read += 1;
                if reader.read_bit()? {
                    state.w = -state.w;
                }
            }
            if state.x > 0 {
                bits_read += 1;
                if reader.read_bit()? {
                    state.x = -state.x;
                }
            }
            if state.y > 0 {
                bits_read += 1;
                if reader.read_bit()? {
                    state.y = -state.y;
                }
            }
        } else {
            if huffman_table.linbits > 0 && state.x == 15 {
                bits_read += huffman_table.linbits;
                // TODO(Herschel): u32?
                state.x += reader.read::<u32>(huffman_table.linbits as u32)? as i32;
            }

            if state.x > 0 {
                bits_read += 1;
                if reader.read_bit()? {
                    state.x = -state.x;
                }
            }

            if huffman_table.linbits > 0 && state.y == 15 {
                bits_read += huffman_table.linbits;
                state.y += reader.read::<u32>(huffman_table.linbits as u32)? as i32;
            }

            if state.y > 0 {
                bits_read += 1;
                if reader.read_bit()? {
                    state.y = -state.y;
                }
            }
        }
    } else {
        *state = Default::default();
    }
    Ok(bits_read)
}
