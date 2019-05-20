use crate::decoder::{BlockType, FrameHeader, GranuleSideInfo, MainDataGranule};
use crate::tables::SCALE_FACTOR_BAND_INDICES;
use std::f32::consts::FRAC_1_SQRT_2;

#[allow(clippy::unreadable_literal)]
const RATIOS: [f32; 6] = [0.0, 0.267949, 0.577350, 1.0, 1.732051, 3.732051];

pub fn stereo(
    header: &FrameHeader,
    side_info: &GranuleSideInfo,
    intensity_stereo: bool,
    mid_side_stereo: bool,
    main_data: &mut MainDataGranule,
) {
    if mid_side_stereo {
        let max_pos = u32::max(main_data.channels[0].count1, main_data.channels[1].count1) as usize;

        for i in 0..max_pos {
            let left = (main_data.channels[0].samples[i] + main_data.channels[1].samples[i])
                * FRAC_1_SQRT_2;
            let right = (main_data.channels[0].samples[i] - main_data.channels[1].samples[i])
                * FRAC_1_SQRT_2;
            main_data.channels[0].samples[i] = left;
            main_data.channels[1].samples[i] = right;
        }
    }

    if intensity_stereo {
        let band_indices = &SCALE_FACTOR_BAND_INDICES[header.sample_rate_table];
        let block_type = side_info.channels[0].block_type;
        if block_type == BlockType::Short || block_type == BlockType::Mixed {
            if block_type == BlockType::Mixed {
                for sfb in 0..8 {
                    if band_indices.0[sfb] >= main_data.channels[1].count1 {
                        stereo_instensity_long(header, sfb, main_data);
                    }
                }

                for sfb in 3..12 {
                    if band_indices.1[sfb] * 3 >= main_data.channels[1].count1 {
                        stereo_instensity_short(header, sfb, main_data);
                    }
                }
            } else {
                for sfb in 0..12 {
                    if band_indices.1[sfb] * 3 >= main_data.channels[1].count1 {
                        stereo_instensity_short(header, sfb, main_data);
                    }
                }
            }
        } else {
            for sfb in 0..21 {
                if band_indices.0[sfb] >= main_data.channels[1].count1 {
                    stereo_instensity_long(header, sfb, main_data);
                }
            }
        }
    }
}

fn stereo_instensity_long(header: &FrameHeader, sfb: usize, main_data: &mut MainDataGranule) {
    let band_indices = &SCALE_FACTOR_BAND_INDICES[header.sample_rate_table];
    let pos = main_data.channels[0].scalefac_l[sfb] as usize;
    let ratio_l;
    let ratio_r;
    if pos != 7 {
        let sfb_start = band_indices.0[sfb] as usize;
        let sfb_end = band_indices.0[sfb + 1] as usize;

        if pos == 6 {
            ratio_l = 1.0;
            ratio_r = 0.0;
        } else {
            ratio_l = RATIOS[pos] / (1.0 + RATIOS[pos]);
            ratio_r = 1.0 / (1.0 + RATIOS[pos]);
        }

        for i in sfb_start..sfb_end {
            let left = ratio_l * main_data.channels[0].samples[i];
            let right = ratio_r * main_data.channels[0].samples[i];
            main_data.channels[0].samples[i] = left;
            main_data.channels[1].samples[i] = right;
        }
    }
}

fn stereo_instensity_short(header: &FrameHeader, sfb: usize, main_data: &mut MainDataGranule) {
    let band_indices = &SCALE_FACTOR_BAND_INDICES[header.sample_rate_table];
    let window_len = (band_indices.1[sfb + 1] - band_indices.1[sfb]) as usize;

    let mut ratio_l: f32;
    let mut ratio_r: f32;

    for win in 0..3 {
        let is_pos = main_data.channels[0].scalefac_s[sfb][win] as usize;
        if is_pos != 7 {
            let sfb_start = band_indices.1[sfb] as usize * 3 + window_len * win;
            let sfb_end = sfb_start + window_len;
            if is_pos == 6 {
                ratio_l = 1.0;
                ratio_r = 0.0;
            } else {
                ratio_l = RATIOS[is_pos] / (1.0 + RATIOS[is_pos]);
                ratio_r = 1.0 / (1.0 + RATIOS[is_pos]);
            }

            for i in sfb_start..sfb_end {
                let left = ratio_l * main_data.channels[0].samples[i];
                let right = ratio_r * main_data.channels[0].samples[i];
                main_data.channels[0].samples[i] = left;
                main_data.channels[1].samples[i] = right;
            }
        }
    }
}
