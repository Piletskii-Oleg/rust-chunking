use crate::{Chunk, SizeParams};

const KB: usize = 1024;
const MIN_CHUNK_SIZE: usize = 2 * KB;
const NORMAL_CHUNK_SIZE: usize = MIN_CHUNK_SIZE + 8 * KB;
const MAX_CHUNK_SIZE: usize = 64 * KB;

const WINDOW_SIZE: usize = 8;

const MASK_S: usize = 0x2F;
const MASK_L: usize = 0x2C;

const LEST: usize = 64;

pub struct Chunker<'a> {
    buf: &'a [u8],
    buf_len: usize,
    out_window: [u8; WINDOW_SIZE],
    in_window: [u8; WINDOW_SIZE],
    normal_size: usize,
    start: usize,
    chk_len: usize,
    distance: usize,
    equal_window_count: usize,
    sizes: SizeParams,
}

impl<'a> Chunker<'a> {
    pub fn default_sizes() -> SizeParams {
        SizeParams {
            min: MIN_CHUNK_SIZE,
            avg: NORMAL_CHUNK_SIZE,
            max: MAX_CHUNK_SIZE,
        }
    }

    pub fn new(buf: &'a [u8], sizes: SizeParams) -> Self {
        Self {
            buf,
            buf_len: buf.len(),
            out_window: [0u8; WINDOW_SIZE],
            in_window: [0u8; WINDOW_SIZE],
            normal_size: sizes.avg,
            start: 0,
            chk_len: sizes.min,
            distance: 0,
            equal_window_count: 0,
            sizes,
        }
    }

    pub fn generate_chunks(&mut self) -> Vec<Chunk> {
        let mut chunks: Vec<Chunk> = vec![];
        self.normal_size = self.sizes.avg;
        if self.buf_len <= self.sizes.min {
            return vec![Chunk::new(0, self.buf_len)];
        }

        if self.buf_len <= self.normal_size {
            self.normal_size = self.buf_len;
        }

        while self.start + self.chk_len < self.buf_len {
            let chunk = self.generate_chunk();

            if !chunks.is_empty() {
                let last = chunks.len() - 1;
                assert_eq!(chunks[last].pos + chunks[last].len, chunk.pos);
            }

            chunks.push(chunk);
        }

        if self.start + self.chk_len >= self.buf_len && self.start != self.buf_len {
            self.chk_len = self.buf_len - self.start;
            chunks.push(Chunk::new(self.start, self.chk_len));
        }

        chunks
    }

    fn generate_chunk(&mut self) -> Chunk {
        if let Some(chunk) = self.check_border() {
            return chunk;
        }

        self.out_window
            .copy_from_slice(&self.buf[self.start..self.start + 8]);
        self.chk_len += 8;
        self.calculate_new_distance();

        if let Some(chunk) = self.try_get_chunk(self.normal_size, MASK_S) {
            return chunk;
        }

        if let Some(chunk) = self.try_get_chunk(self.sizes.max, MASK_L) {
            return chunk;
        }

        self.make_chunk(0)
    }

    fn try_get_chunk(&mut self, size_limit: usize, mask: usize) -> Option<Chunk> {
        while self.chk_len < size_limit {
            if let Some(chunk) = self.check_border() {
                return Some(chunk);
            }

            self.in_window.copy_from_slice(
                &self.buf[self.start + self.chk_len..self.start + self.chk_len + 8],
            );

            if self.in_window == self.out_window {
                self.equal_window_count += 1;
                if self.equal_window_count == LEST {
                    return Some(self.make_chunk(8));
                } else {
                    self.chk_len += 8;
                    continue;
                }
            }

            self.equal_window_count = 0;
            if let Some(chunk) = self.try_extract(mask) {
                return Some(chunk);
            }

            self.out_window.copy_from_slice(&self.in_window);
            self.chk_len += 8;
        }
        None
    }

    fn try_extract(&mut self, mask: usize) -> Option<Chunk> {
        for j in 0..WINDOW_SIZE {
            if (self.distance & mask) == 0 {
                return Some(self.make_chunk(8));
            }

            // self.distance = (self.distance << 1) + DISTANCE_MAP[BYTE][self.in_window[j] as usize];
            self.slide_one_byte(j);
        }
        None
    }

    fn calculate_new_distance(&mut self) {
        self.distance = self
            .out_window
            .iter()
            .map(|&byte| BYTE_DISTANCES[byte as usize])
            .sum();
    }

    fn slide_one_byte(&mut self, index: usize) {
        let old = self.out_window[index];
        let new = self.in_window[index];

        self.distance += BYTE_DISTANCES[new as usize];
        self.distance -= BYTE_DISTANCES[old as usize];
    }

    fn make_chunk(&mut self, add_len: usize) -> Chunk {
        self.chk_len += add_len;

        let pos = self.start;
        let len = self.chk_len;

        self.start += self.chk_len;
        self.chk_len = self.sizes.min;

        Chunk::new(pos, len)
    }

    fn check_border(&mut self) -> Option<Chunk> {
        if self.start + self.chk_len + 8 >= self.buf_len {
            let pos = self.start;
            let len = self.buf_len - self.start;

            self.start = self.buf_len;

            Some(Chunk::new(pos, len))
        } else {
            None
        }
    }
}

impl Iterator for Chunker<'_> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.buf_len {
            None
        } else {
            Some(self.generate_chunk())
        }
    }
}

const BYTE_DISTANCES: [usize; 256] = [
    4, 5, 3, 4, 5, 6, 4, 5, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 6, 7, 5, 6, 4, 5, 3, 4, 5, 6, 4, 5,
    3, 4, 2, 3, 4, 5, 3, 4, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 3, 4, 2, 3, 4, 5, 3, 4,
    5, 6, 4, 5, 6, 7, 5, 6, 4, 5, 3, 4, 5, 6, 4, 5, 6, 7, 5, 6, 7, 8, 6, 7, 5, 6, 4, 5, 6, 7, 5, 6,
    4, 5, 3, 4, 5, 6, 4, 5, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 6, 7, 5, 6, 4, 5, 3, 4, 5, 6, 4, 5,
    3, 4, 2, 3, 4, 5, 3, 4, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 3, 4, 2, 3, 4, 5, 3, 4,
    2, 3, 1, 2, 3, 4, 2, 3, 1, 2, 0, 1, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 2, 3, 1, 2, 3, 4, 2, 3,
    4, 5, 3, 4, 5, 6, 4, 5, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 6, 7, 5, 6, 4, 5, 3, 4, 5, 6, 4, 5,
    3, 4, 2, 3, 4, 5, 3, 4, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 3, 4, 2, 3, 4, 5, 3, 4,
];
