use crate::tables::SCALE_FACTOR_BAND_INDICES;
use crate::types::{BlockType, FrameHeader, GranuleChannelSideInfo, MainDataChannel};

pub fn requantize(
    header: &FrameHeader,
    side_info: &GranuleChannelSideInfo,
    main_data: &mut MainDataChannel,
) {
    if side_info.block_type == BlockType::Short || side_info.block_type == BlockType::Mixed {
        if side_info.block_type == BlockType::Mixed {
            let mut sfb = 0;
            let mut next_sfb = SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].0[sfb + 1];
            for i in 0..36 {
                if i == next_sfb {
                    sfb += 1;
                    next_sfb = SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].0[sfb + 1];
                }

                requantize_long(side_info, i as usize, sfb, main_data);
            }

            sfb = 3;
            next_sfb = SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1[sfb + 1] * 3;
            let mut window_len = SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1[sfb + 1]
                - SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1[sfb];

            let mut i = 36;
            while i < main_data.count1 {
                if i == next_sfb {
                    assert!(sfb < 14);
                    sfb += 1;
                    next_sfb = SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1[sfb + 1] * 3;
                    window_len = SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1[sfb + 1]
                        - SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1[sfb];
                }

                for win in 0..3 {
                    for _ in 0..window_len {
                        requantize_short(
                            side_info,
                            i as usize,
                            sfb,
                            win,
                            &side_info.subblock_gain[..],
                            main_data,
                        );
                        i += 1;
                    }
                }
            }
        } else {
            // Data only contains short blocks.
            let mut sfb = 0;
            let mut next_sfb = SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1[sfb + 1] * 3;
            let mut window_len = SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1[sfb + 1]
                - SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1[sfb];

            let mut i = 0;
            while i < main_data.count1 {
                if i == next_sfb {
                    assert!(sfb < 14);
                    sfb += 1;
                    next_sfb = SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1[sfb + 1] * 3;
                    window_len = SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1[sfb + 1]
                        - SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1[sfb];
                }

                for win in 0..3 {
                    for _ in 0..window_len {
                        requantize_short(
                            side_info,
                            i as usize,
                            sfb,
                            win,
                            &side_info.subblock_gain[..],
                            main_data,
                        );
                        i += 1;
                    }
                }
            }
        }
    } else {
        // Data contains only long blocks.
        let mut sfb = 0;
        let mut next_sfb = SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].0[sfb + 1];

        for i in 0..main_data.count1 {
            if i == next_sfb {
                assert!(sfb < 23);
                sfb += 1;
                next_sfb = SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].0[sfb + 1];
            }

            requantize_long(side_info, i as usize, sfb, main_data);
        }
    }
}

// Requnaitze subband using long blocks.
fn requantize_long(
    side_info: &GranuleChannelSideInfo,
    pos: usize,
    sfb: usize,
    main_data: &mut MainDataChannel,
) {
    const PRE_TAB: [f32; 22] = [
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 2.0, 2.0, 3.0,
        3.0, 3.0, 2.0, 0.0,
    ];

    assert!(pos < 576);
    let sf_mult = if side_info.scalefac_scale { 1.0 } else { 0.5 };
    let pf_x_pt = if side_info.preflag { PRE_TAB[sfb] } else { 0.0 };
    let tmp1 = f64::powf(
        2.0,
        -sf_mult * (f64::from(main_data.scalefac_l[sfb]) + f64::from(pf_x_pt)),
    );
    let tmp2 = f64::powf(2.0, 0.25 * (f64::from(side_info.global_gain) - 210.0));
    let tmp3 = if main_data.samples[pos] < 0.0 {
        -requantize_pow_43(-main_data.samples[pos])
    } else {
        requantize_pow_43(main_data.samples[pos])
    };

    main_data.samples[pos] = (tmp1 * tmp2 * f64::from(tmp3)) as f32;
}

// Requanitze short block subband.
fn requantize_short(
    side_info: &GranuleChannelSideInfo,
    pos: usize,
    sfb: usize,
    window: usize,
    subblock_gain: &[f32],
    data: &mut MainDataChannel,
) {
    assert!(pos < 576);
    let sf_mult = if side_info.scalefac_scale { 1.0 } else { 0.5 };
    let tmp1 = f64::powf(2.0, -sf_mult * f64::from(data.scalefac_s[sfb][window]));
    let tmp2 = f64::powf(
        2.0,
        0.25 * (f64::from(side_info.global_gain) - 210.0 - 8.0 * f64::from(subblock_gain[window])),
    );
    let tmp3 = if data.samples[pos] < 0.0 {
        -requantize_pow_43(-data.samples[pos])
    } else {
        requantize_pow_43(data.samples[pos])
    };
    data.samples[pos] = (tmp1 * tmp2 * f64::from(tmp3)) as f32;
}

fn requantize_pow_43(sample: f32) -> f32 {
    f32::powf(f32::trunc(sample), 4.0 / 3.0)
}

pub fn reorder(
    header: &FrameHeader,
    side_info: &GranuleChannelSideInfo,
    main_data: &mut MainDataChannel,
) {
    let mut reorder_buffer = [0f32; 576];

    let band_indices = &SCALE_FACTOR_BAND_INDICES[header.sample_rate_table].1;
    if side_info.block_type == BlockType::Short || side_info.block_type == BlockType::Mixed {
        let mut sfb = if side_info.block_type == BlockType::Mixed {
            3
        } else {
            0
        };
        let mut next_sfb = band_indices[sfb + 1] * 3;
        let mut window_len = (band_indices[sfb + 1] - band_indices[sfb]) as usize;
        let mut i = if sfb == 0 { 0 } else { 36 };
        while i < 576 {
            if i == next_sfb {
                for (j, &val) in reorder_buffer[0..3 * window_len].iter().enumerate() {
                    main_data.samples[3 * band_indices[sfb] as usize + j] = val;
                }

                if i >= main_data.count1 {
                    return;
                }

                sfb += 1;
                next_sfb = band_indices[sfb + 1] * 3;
                window_len = (band_indices[sfb + 1] - band_indices[sfb]) as usize;
            }

            for win in 0..3 {
                for j in 0..window_len {
                    reorder_buffer[j * 3 + win] = main_data.samples[i as usize];
                    i += 1;
                }
            }
        }
    }
}
