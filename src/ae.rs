use crate::{Chunk, SizeParams};

const MIN_CHUNK_SIZE: usize = 1024 * 8;
const MAX_CHUNK_SIZE: usize = 1024 * 16;

pub struct Chunker<'a> {
    buf: &'a [u8],
    len: usize,
    pos: usize,
    chunk_start: usize,
    sizes: SizeParams,
    max_value: u8,
    max_position: usize,
    window_size: usize,
}
impl<'a> Chunker<'a> {
    pub fn default_sizes() -> SizeParams {
        SizeParams {
            min: MIN_CHUNK_SIZE,
            avg: (MAX_CHUNK_SIZE + MIN_CHUNK_SIZE) / 2,
            max: MAX_CHUNK_SIZE,
        }
    }

    pub fn new(buf: &'a [u8], sizes: SizeParams) -> Self {
        Chunker {
            buf,
            len: buf.len(),
            pos: 0,
            chunk_start: 0,
            sizes,
            max_value: 0,
            max_position: 0,
            window_size: 32,
        }
    }
    
    fn find_border(&mut self) -> Option<usize> {
        if self.len == self.pos {
            return None;
        }

        if self.len - self.pos < self.sizes.min {
            self.pos = self.len;
            return Some(self.pos);
        }

        self.pos += 1;
        self.max_value = self.buf[self.pos];
        self.max_position = self.pos;

        while self.pos < self.len {
            if self.pos - self.chunk_start > self.sizes.max {
                return Some(self.pos);
            }

            if self.buf[self.pos] < self.max_value {
                if self.pos == self.max_position + self.window_size {
                    return Some(self.pos);
                }
            } else {
                self.max_value = self.buf[self.pos];
                self.max_position = self.pos;
            }

            self.pos += 1;
        }

        Some(self.pos)
    }
}


impl Iterator for Chunker<'_> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        let next_pos = self.find_border()?;

        let start = self.chunk_start;
        let length = next_pos - self.chunk_start;
        self.chunk_start = next_pos;

        Some(Chunk::new(start, length))
    }
}