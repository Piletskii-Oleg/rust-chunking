use crate::Chunk;

const KB: usize = 1024;
const MIN_CHUNK_SIZE: usize = 2 * KB;
const NORMAL_CHUNK_SIZE: usize = MIN_CHUNK_SIZE + 8 * KB;
const MAX_CHUNK_SIZE: usize = 64 * KB;

const WINDOW_SIZE: usize = 8;

const PATTERN: u128 = 170170170170170170170170;
const MASK_S: usize = 0x2F;
const MASK_L: usize = 0x2C;

const LEST: usize = 64;

#[derive(Debug)]
enum HammingError {
    DifferentLength,
}

pub struct Chunker {
    out_window: [u8; WINDOW_SIZE],
    out_window_idx: usize,
    in_window: [u8; WINDOW_SIZE],
    distance_map: Vec<Vec<usize>>,
    normal_size: usize,
    start: usize,
    chk_len: usize,
    distance: usize,
    equal_window_count: usize
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

fn distance_map_2() -> Vec<Vec<usize>> {
    let hex_numbers = (0u8..=255u8).map(byte_to_hex).collect::<Vec<String>>();
    hex_numbers
        .iter()
        .map(|byte| {
            let hexes = (0u8..=255u8).map(byte_to_hex).collect::<Vec<String>>();
            hexes
                .iter()
                .map(|this_byte| hamming_distance(byte, this_byte).unwrap())
                .collect::<Vec<usize>>()
        })
        .collect()
}

impl Chunker {
    pub fn new() -> Self {
        Self {
            out_window: [0u8; WINDOW_SIZE],
            out_window_idx: MIN_CHUNK_SIZE,
            in_window: [0u8; WINDOW_SIZE],
            distance_map: distance_map(),
            normal_size: NORMAL_CHUNK_SIZE,
            start: MIN_CHUNK_SIZE,
            chk_len: MIN_CHUNK_SIZE,
            distance: 0,
            equal_window_count: 0
        }
    }

    pub fn generate_chunks(&mut self, data: &[u8]) -> Vec<Chunk> {
        let mut chunks = vec![];
        self.normal_size = NORMAL_CHUNK_SIZE;
        if data.len() <= MIN_CHUNK_SIZE {
            return vec![Chunk::new(0, data.len())];
        }

        if data.len() <= self.normal_size {
            self.normal_size = data.len();
        }

        while self.start + self.chk_len < data.len() {
            let chunk = self.generate_chunk(data);
            chunks.push(chunk);
        }

        chunks
    }

    fn generate_chunk(&mut self, data: &[u8]) -> Chunk {
        const BYTE: usize = 0xAA;

        if self.start + self.chk_len >= data.len() {
            self.chk_len = data.len() - self.start;
            return Chunk::new(self.start, self.chk_len);
        }

        self.out_window
            .copy_from_slice(&data[self.start..self.start + 8]);
        self.out_window_idx = self.start + 8;

        self.chk_len += 8;

        if let Some(chunk) = self.try_get_chunk(&data, self.normal_size, MASK_S) {
            return chunk;
        }

        if let Some(chunk) = self.try_get_chunk(&data, MAX_CHUNK_SIZE, MASK_L) {
            return chunk;
        }

        let len = self.chk_len;
        let pos = self.start;

        self.start += self.chk_len + MIN_CHUNK_SIZE;
        self.chk_len = MIN_CHUNK_SIZE;

        Chunk::new(pos, len)
    }

    fn try_get_chunk(&mut self, data: &[u8], size_limit: usize, mask: usize) -> Option<Chunk> {
        while self.chk_len < size_limit {
            if self.start + self.chk_len + 8 >= data.len() {
                self.chk_len = data.len() - self.start;
                return Some(Chunk::new(self.start, self.chk_len));
            }

            self.distance = self
                .out_window
                .iter()
                .map(|&byte|  self.distance_map[0xAA][byte as usize])
                .sum();

            self.in_window
                .copy_from_slice(&data[self.start + self.chk_len..self.start + self.chk_len + 8]);

            if self.in_window == self.out_window {
                self.equal_window_count += 1;
                if self.equal_window_count == LEST {
                    self.chk_len += 8;

                    let len = self.chk_len;
                    let pos = self.start;

                    self.start += self.chk_len + MIN_CHUNK_SIZE;
                    self.chk_len = MIN_CHUNK_SIZE;

                    return Some(Chunk::new(pos, len));
                } else {
                    self.chk_len += 8;
                    continue;
                }
            }

            self.equal_window_count = 0;
            for j in 0..8 {
                if (self.distance & mask) == 0 {
                    self.chk_len += 8;

                    let pos = self.start;
                    let len = self.chk_len;

                    self.start += self.chk_len + MIN_CHUNK_SIZE;
                    self.chk_len = MIN_CHUNK_SIZE;

                    return Some(Chunk::new(pos, len));
                }
                self.slide_one_byte(data, j);
            }

            self.out_window.copy_from_slice(&self.in_window[..]);
            self.chk_len += 8;
        }
        None
    }

    fn slide_one_byte(&mut self, data: &[u8], index: usize) {
        const BYTE: usize = 0xAA;

        let old = self.out_window[0];
        self.out_window.copy_within(1.., 0);

        let new = data[self.start + self.chk_len + index];
        self.out_window[WINDOW_SIZE - 1] = new;

        self.distance += self.distance_map[BYTE][new as usize];
        self.distance -= self.distance_map[BYTE][old as usize];
    }
}

fn byte_to_hex(byte: u8) -> String {
    format!("{:02X}", byte)
}

fn hamming_distance(str1: &str, str2: &str) -> Result<usize, HammingError> {
    if str1.len() != str2.len() {
        Err(HammingError::DifferentLength)
    } else {
        let same = str1
            .chars()
            .zip(str2.chars())
            .filter(|&(a, b)| a == b)
            .count();

        Ok(str1.len() - same)
    }
}
