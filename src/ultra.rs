use crate::Chunk;

const KB: usize = 1024;
const MIN_CHUNK_SIZE: usize = 2 * KB;
const NORMAL_CHUNK_SIZE: usize = MIN_CHUNK_SIZE + 8 * KB;
const MAX_CHUNK_SIZE: usize = 64 * KB;

const WINDOW_SIZE: usize = 8;

const BYTE: usize = 0xAA;
const MASK_S: usize = 0x2F;
const MASK_L: usize = 0x2C;

const LEST: usize = 64;

#[derive(Debug)]
enum HammingError {
    DifferentLength,
}

pub struct Chunker {
    out_window: [u8; WINDOW_SIZE],
    in_window: [u8; WINDOW_SIZE],
    distance_map: Vec<Vec<usize>>,
    normal_size: usize,
    start: usize,
    chk_len: usize,
    distance: usize,
    equal_window_count: usize,
}

fn distance_map() -> Vec<Vec<usize>> {
    (0u8..=255u8)
        .map(|byte| {
            (0u8..=255u8)
                .map(|this_byte| (byte ^ this_byte).count_ones() as usize)
                .collect()
        })
        .collect()
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunker {
    pub fn new() -> Self {
        Self {
            out_window: [0u8; WINDOW_SIZE],
            in_window: [0u8; WINDOW_SIZE],
            distance_map: distance_map(),
            normal_size: NORMAL_CHUNK_SIZE,
            start: 0,
            chk_len: MIN_CHUNK_SIZE,
            distance: 0,
            equal_window_count: 0,
        }
    }

    pub fn generate_chunks(&mut self, data: &[u8]) -> Vec<Chunk> {
        let mut chunks: Vec<Chunk> = vec![];
        self.normal_size = NORMAL_CHUNK_SIZE;
        if data.len() <= MIN_CHUNK_SIZE {
            return vec![Chunk::new(0, data.len())];
        }

        if data.len() <= self.normal_size {
            self.normal_size = data.len();
        }

        while self.start + self.chk_len < data.len() {
            let chunk = self.generate_chunk(data);

            if !chunks.is_empty() {
                let last = chunks.len() - 1;
                assert_eq!(chunks[last].pos + chunks[last].len, chunk.pos);
            }

            chunks.push(chunk);
        }

        if self.start + self.chk_len >= data.len() && self.start != data.len() {
            self.chk_len = data.len() - self.start;
            chunks.push(Chunk::new(self.start, self.chk_len));
        }

        chunks
    }

    fn generate_chunk(&mut self, data: &[u8]) -> Chunk {
        if let Some(chunk) = self.check_border(data) {
            return chunk;
        }

        self.out_window
            .copy_from_slice(&data[self.start..self.start + 8]);
        self.chk_len += 8;
        self.calculate_new_distance();

        if let Some(chunk) = self.try_get_chunk(data, self.normal_size, MASK_S) {
            return chunk;
        }

        if let Some(chunk) = self.try_get_chunk(data, MAX_CHUNK_SIZE, MASK_L) {
            return chunk;
        }

        self.make_chunk(0)
    }

    fn try_get_chunk(&mut self, data: &[u8], size_limit: usize, mask: usize) -> Option<Chunk> {
        while self.chk_len < size_limit {
            if let Some(chunk) = self.check_border(data) {
                return Some(chunk);
            }

            self.in_window
                .copy_from_slice(&data[self.start + self.chk_len..self.start + self.chk_len + 8]);

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
        for j in 0..8 {
            if (self.distance & mask) == 0 {
                return Some(self.make_chunk(8));
            }
            self.slide_one_byte(j);
        }
        None
    }

    fn calculate_new_distance(&mut self) {
        self.distance = self
            .out_window
            .iter()
            .map(|&byte| self.distance_map[BYTE][byte as usize])
            .sum();
    }

    fn slide_one_byte(&mut self, index: usize) {
        let old = self.out_window[index];
        let new = self.in_window[index];

        self.distance += self.distance_map[BYTE][new as usize];
        self.distance -= self.distance_map[BYTE][old as usize];
    }

    fn make_chunk(&mut self, add_len: usize) -> Chunk {
        self.chk_len += add_len;

        let pos = self.start;
        let len = self.chk_len;

        self.start += self.chk_len;
        self.chk_len = MIN_CHUNK_SIZE;

        Chunk::new(pos, len)
    }

    fn check_border(&mut self, data: &[u8]) -> Option<Chunk> {
        if self.start + self.chk_len >= data.len() {
            let pos = self.start;
            let len = data.len() - self.start;

            self.start = data.len();

            Some(Chunk::new(pos, len))
        } else {
            None
        }
    }
}
