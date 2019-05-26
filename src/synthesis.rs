use crate::tables::{COS_N12, COS_N36, IMDCT_WIN, SBS_N_WIN, SYNTH_DTBL};
use crate::types::{BlockType, GranuleChannelSideInfo};

#[allow(clippy::unreadable_literal)]
pub fn antialias(side_info: &GranuleChannelSideInfo, samples: &mut [f32; 576]) {
    const CS: [f32; 8] = [
        0.857493, 0.881742, 0.949629, 0.983315, 0.995518, 0.999161, 0.999899, 0.999993,
    ];
    const CA: [f32; 8] = [
        -0.514496, -0.471732, -0.313377, -0.181913, -0.094574, -0.040966, -0.014199, -0.003700,
    ];

    let sblim = if side_info.block_type == BlockType::Short {
        // No anti-aliasing done for short blocks.
        return;
    } else if side_info.block_type == BlockType::Mixed {
        2
    } else {
        32
    };

    for sb in 1..sblim {
        for i in 0..8 {
            let li = 18 * sb - 1 - i;
            let ui = 18 * sb + i;
            let lb = samples[li] * CS[i] - samples[ui] * CA[i];
            let ub = samples[ui] * CS[i] + samples[li] * CA[i];
            samples[li] = lb;
            samples[ui] = ub;
        }
    }
}

pub(crate) fn hybrid_synthesis(
    block_type: BlockType,
    store: &mut [[f32; 18]; 32],
    samples: &mut [f32; 576],
) {
    for sb in 0..32 {
        let block_type = match block_type {
            BlockType::Long => 0,
            BlockType::Start => 1,
            BlockType::Short => 2,
            BlockType::Mixed => {
                if sb < 2 {
                    0
                } else {
                    2
                }
            }
            BlockType::End => 3,
        };

        let out = imdct_win(block_type, &samples[sb * 18..sb * 18 + 18]);
        for i in 0..18 {
            samples[sb * 18 + i] = out[i] + store[sb][i];
            store[sb][i] = out[i + 18];
        }
    }
}

fn imdct_win(block_type: usize, samples: &[f32]) -> [f32; 36] {
    let mut out = [0f32; 36];
    let imdct_table = &IMDCT_WIN[block_type];
    if block_type == 2 {
        for i in 0..3 {
            for p in 0..12 {
                let mut sum = 0.0;
                for m in 0..6 {
                    sum += samples[i + 3 * m] * COS_N12[m][p];
                }
                out[6 * i + p + 6] = sum * imdct_table[p];
            }
        }
    } else {
        for p in 0..36 {
            let mut sum = 0.0;
            for m in 0..18 {
                sum += samples[m] * COS_N36[m][p];
            }

            out[p] = sum * imdct_table[p];
        }
    }
    out
}

pub fn frequency_inversion(samples: &mut [f32; 576]) {
    for sb in (1..32).step_by(2) {
        for i in (1..18).step_by(2) {
            let n = sb * 18 + i;
            samples[n] = -samples[n];
        }
    }
}

pub fn subband_synthesis(samples: &[f32; 576], v_vec: &mut [f32; 1024], out: &mut [f32]) {
    let mut s_vec = [0f32; 32];
    let mut u_vec = [0f32; 512];

    for ss in 0..18 {
        for i in (64..=1023).rev() {
            v_vec[i] = v_vec[i - 64];
        }

        for i in 0..32 {
            s_vec[i] = samples[i * 18 + ss];
        }

        for (i, row) in SBS_N_WIN.iter().enumerate() {
            let mut sum = 0.0;
            for (j, &sbs_n_win) in row.iter().enumerate() {
                sum += sbs_n_win * s_vec[j];
            }
            v_vec[i] = sum;
        }

        for i in 0..8 {
            for j in 0..32 {
                let i6 = i << 6;
                let i7 = i << 7;

                u_vec[i6 + j] = v_vec[i7 + j];
                u_vec[i6 + j + 32] = v_vec[i7 + j + 96];
            }
        }

        for i in 0..512 {
            u_vec[i] *= SYNTH_DTBL[i];
        }

        for i in 0..32 {
            let mut sum = 0.0;
            for j in 0..16 {
                sum += u_vec[(j << 5) + i];
            }
            out[(32 * ss) + i] = sum;
        }
    }
}
