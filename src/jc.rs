use crate::{Chunk, SizeParams};
use gearhash::DEFAULT_TABLE;

const KB: usize = 1024;

const MIN_SIZE: usize = 512;
const AVG_SIZE: usize = 8 * KB;
const MAX_SIZE: usize = 16 * KB;

const GEAR_TABLE: &gearhash::Table = &DEFAULT_TABLE;

const fn low_bits_mask(ones: u32) -> u64 {
    if ones == 0 {
        0
    } else if ones >= 64 {
        u64::MAX
    } else {
        (1u64 << ones) - 1
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct JcParams {
    pub mask_c: u64,
    pub mask_j: u64,
    pub jump_len: usize,
}

impl JcParams {
    /// c_ones = log2(c_avg) - 1, j_ones = log2(c_avg) - 2, js = c_avg/2.
    pub fn from_sizes(sizes: SizeParams) -> Self {
        let avg_pow2 = sizes.avg.next_power_of_two().max(2);
        let log2 = (usize::BITS - 1) - avg_pow2.leading_zeros();

        let c_ones = log2.saturating_sub(1);
        let j_ones = log2.saturating_sub(2);

        let mask_c = low_bits_mask(c_ones);
        let mask_j = low_bits_mask(j_ones);

        let jump_len = sizes.avg / 2;

        Self {
            mask_c,
            mask_j,
            jump_len: jump_len.max(1),
        }
    }
}

pub struct Chunker<'a> {
    buf: &'a [u8],
    pos: usize,
    len: usize,
    sizes: SizeParams,
    params: JcParams,
}

impl<'a> Chunker<'a> {
    pub fn default_sizes() -> SizeParams {
        SizeParams {
            min: MIN_SIZE,
            avg: AVG_SIZE,
            max: MAX_SIZE,
        }
    }

    pub fn new(buf: &'a [u8], sizes: SizeParams) -> Self {
        let params = JcParams::from_sizes(sizes);
        Self::with_params(buf, sizes, params)
    }

    pub fn with_params(buf: &'a [u8], sizes: SizeParams, params: JcParams) -> Self {
        debug_assert!(sizes.min <= sizes.avg && sizes.avg <= sizes.max);
        Self {
            buf,
            pos: 0,
            len: buf.len(),
            sizes,
            params,
        }
    }

    fn find_border(&mut self) -> Option<usize> {
        if self.pos == self.len {
            return None;
        }

        let remaining = self.len - self.pos;

        if remaining <= self.sizes.min {
            self.pos = self.len;
            return Some(remaining);
        }

        let end_limit = (self.pos + self.sizes.max).min(self.len);

        let mut fp: u64 = 0;
        let mut idx: usize = self.pos;
        let mut chunk_len: usize = 0;

        while idx < end_limit {
            let b = self.buf[idx];
            fp = fp.wrapping_shl(1).wrapping_add(GEAR_TABLE[b as usize]);

            idx += 1;
            chunk_len += 1;

            if chunk_len < self.sizes.min {
                continue;
            }

            if (fp & self.params.mask_j) == 0 {
                if (fp & self.params.mask_c) == 0 {
                    // cut-point
                    self.pos += chunk_len;
                    return Some(chunk_len);
                }

                fp = 0;

                let can_skip = end_limit - idx;
                let skip = self.params.jump_len.min(can_skip);
                idx += skip;
                chunk_len += skip;
            }
        }

        let out_len = end_limit - self.pos;
        self.pos = end_limit;
        Some(out_len)
    }
}

impl<'a> Iterator for Chunker<'a> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.pos;
        self.find_border().map(|length| Chunk::new(start, length))
    }
}
