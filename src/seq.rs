use crate::{Chunk, SizeParams};
use std::cmp::Ordering;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum OperationMode {
    Increasing,
    Decreasing,
}

/// Contains parameters specified in the SeqCDC paper.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Config {
    sequence_length: usize,
    skip_trigger: usize,
    skip_size: usize,
}

pub struct Chunker<'a> {
    buf: &'a [u8],
    len: usize,
    position: usize,
    sizes: SizeParams,
    mode: OperationMode,
    sequence_length: usize,
    skip_trigger: usize,
    skip_size: usize,
}

impl Config {
    pub fn new(sequence_length: usize, skip_trigger: usize, skip_size: usize) -> Self {
        Self {
            sequence_length,
            skip_trigger,
            skip_size,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sequence_length: 5,
            skip_trigger: 55,
            skip_size: 256,
        }
    }
}

impl<'a> Chunker<'a> {
    pub fn default_sizes() -> SizeParams {
        SizeParams {
            min: 4 * 1024,
            avg: 8 * 1024,
            max: 16 * 1024,
        }
    }

    pub fn new(buf: &'a [u8], params: SizeParams, mode: OperationMode, config: Config) -> Self {
        Self {
            buf,
            len: buf.len(),
            position: 0,
            sizes: params,
            mode,
            sequence_length: config.sequence_length,
            skip_trigger: config.skip_trigger,
            skip_size: config.skip_size,
        }
    }

    fn find_border_increasing(&mut self) -> Option<usize> {
        if self.position == self.len {
            return None;
        }

        if self.len - self.position < self.sizes.min {
            let delta = self.len - self.position;
            self.position = self.len;
            return Some(delta);
        }

        self.position += self.sizes.min;

        let mut chunk_len = self.sizes.min;
        let mut sequence_length = 0;
        let mut opposing_slope_count = 0;

        while self.position < self.len && chunk_len < self.sizes.max {
            self.position += 1;
            chunk_len += 1;

            match self.buf[self.position - 1].cmp(&self.buf[self.position - 2]) {
                Ordering::Less => {
                    sequence_length = 0;
                    opposing_slope_count += 1;
                }
                Ordering::Equal => continue,
                Ordering::Greater => sequence_length += 1,
            }

            if sequence_length == self.sequence_length {
                return Some(chunk_len);
            }
            if opposing_slope_count == self.skip_trigger {
                self.position += self.skip_size;
                chunk_len += self.skip_size;
                opposing_slope_count = 0;
            }
        }

        if self.position > self.len {
            let delta = self.position - self.len;
            self.position = self.len;
            chunk_len -= delta;
        }

        Some(chunk_len)
    }

    fn find_border_decreasing(&mut self) -> Option<usize> {
        if self.position == self.len {
            return None;
        }

        if self.len - self.position < self.sizes.min {
            let delta = self.len - self.position;
            self.position = self.len;
            return Some(delta);
        }

        self.position += self.sizes.min;

        let mut chunk_len = self.sizes.min;
        let mut sequence_length = 0;
        let mut opposing_slope_count = 0;

        while self.position < self.len && chunk_len < self.sizes.max {
            self.position += 1;
            chunk_len += 1;

            match self.buf[self.position - 1].cmp(&self.buf[self.position - 2]) {
                Ordering::Less => sequence_length += 1,
                Ordering::Equal => continue,
                Ordering::Greater => {
                    sequence_length = 0;
                    opposing_slope_count += 1
                }
            }

            if sequence_length == self.sequence_length {
                return Some(chunk_len);
            }
            if opposing_slope_count == self.skip_trigger {
                self.position += self.skip_size;
                chunk_len += self.skip_size;
                opposing_slope_count = 0;
            }
        }

        if self.position > self.len {
            let delta = self.position - self.len;
            self.position = self.len;
            chunk_len -= delta;
        }

        Some(chunk_len)
    }

    /// Returns next size of the chunk.
    ///
    /// Reads the info about operation mode from the chunker instance.
    fn find_border(&mut self) -> Option<usize> {
        match self.mode {
            OperationMode::Increasing => self.find_border_increasing(),
            OperationMode::Decreasing => self.find_border_decreasing(),
        }
    }
}

impl Iterator for Chunker<'_> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.position;

        self.find_border().map(|length| Chunk::new(start, length))
    }
}
