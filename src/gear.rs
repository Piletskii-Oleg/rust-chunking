use crate::{Chunk, SizeParams};
use rand::Rng;

const MIN_SIZE: usize = 4 * 1024; // minimal chunk size, 4k
const AVG_SIZE: usize = 8 * 1024; // average chunk size, 8k
const MAX_SIZE: usize = 12 * 1024; // maximum chunk size, 12k

// rolling hash window constants
const WIN_SIZE: usize = 13; // log2(AVG_SIZE)
const WIN_SLIDE_OFFSET: usize = 1;
const WIN_SLIDE_POS: usize = MIN_SIZE - WIN_SLIDE_OFFSET;

const DIGEST_SIZE: usize = 64; // must be 2^n
const MASK: u64 = !0u64 << ((DIGEST_SIZE - WIN_SIZE) as u32);

pub struct Chunker<'a> {
    buf: &'a [u8],
    params: ChunkerParams,
    pos: usize,
    len: usize,
    sizes: SizeParams,
    win_slide_pos: usize,
}

/// Pre-calculated chunker parameters
#[derive(Clone)]
pub struct ChunkerParams {
    gear_table: Vec<u64>, // pre-computed random byte map, length is 256
}

impl<'a> Chunker<'a> {
    pub fn default_sizes() -> SizeParams {
        SizeParams {
            min: MIN_SIZE,
            avg: AVG_SIZE,
            max: MAX_SIZE,
        }
    }

    pub fn new(buf: &[u8]) -> Chunker {
        Chunker {
            buf,
            pos: 0,
            params: ChunkerParams::new(),
            len: buf.len(),
            sizes: Self::default_sizes(),
            win_slide_pos: WIN_SLIDE_POS,
        }
    }

    pub fn with_params(buf: &'a [u8], params: ChunkerParams, sizes: SizeParams) -> Self {
        debug_assert!(sizes.min > WIN_SLIDE_OFFSET);

        Self {
            buf,
            params,
            pos: 0,
            len: buf.len(),
            sizes,
            win_slide_pos: sizes.min - WIN_SLIDE_OFFSET,
        }
    }

    fn find_border(&mut self) -> Option<usize> {
        if self.len == self.pos {
            return None;
        }

        if self.len - self.pos < self.sizes.min {
            let pos = self.pos;
            self.pos = self.len;
            return Some(self.len - pos);
        }

        self.pos += self.win_slide_pos;
        let mut chunk_len = self.win_slide_pos;

        let mut roll_hash: u64 = 0;

        while self.pos < self.len {
            let ch = self.buf[self.pos];
            let gear_value = self.params.gear_table[ch as usize];

            roll_hash = (roll_hash << 1).wrapping_add(gear_value);

            chunk_len += 1;
            self.pos += 1;

            if (roll_hash & MASK) == 0 || chunk_len >= self.sizes.max {
                return Some(chunk_len);
            }
        }

        Some(chunk_len)
    }

    pub fn give_params(self) -> ChunkerParams {
        self.params
    }
}

impl Iterator for Chunker<'_> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.pos;

        self.find_border().map(|length| Chunk::new(start, length))
    }
}

impl ChunkerParams {
    pub fn new() -> Self {
        let mut cp = ChunkerParams::default();
        let mut rng = rand::thread_rng();

        for i in 0..256 {
            cp.gear_table[i] = rng.gen::<u64>();
        }

        cp
    }
}

impl Default for ChunkerParams {
    fn default() -> Self {
        let mut ret = ChunkerParams {
            gear_table: vec![0u64; 256],
        };
        ret.gear_table.shrink_to_fit();
        ret
    }
}

#[cfg(test)]
mod tests {
    use crate::gear::{Chunker, ChunkerParams};
    use crate::SizeParams;

    #[test]
    fn gear_works_with_different_sizes() {
        let sizes = SizeParams::new(3000, 50000, 100000);

        let data = vec![3; 1024 * 1024 * 300];

        let chunker = Chunker::with_params(&data, ChunkerParams::default(), sizes);
        for chunk in chunker {
            println!("{:?}", chunk);
        }
    }
}
