use crate::{Chunk, SizeParams};

const MIN_CHUNK_SIZE: usize = 1024 * 8;
const MAX_CHUNK_SIZE: usize = 1024 * 16;

const WINDOW_PRIMARY_COUNT: usize = 22;
const WINDOW_SECONDARY_COUNT: usize = 2;
const WINDOW_COUNT: usize = WINDOW_PRIMARY_COUNT + WINDOW_SECONDARY_COUNT;

const WINDOW_SIZE: usize = 180;
const WINDOW_MATRIX_SHIFT: usize = 42; // WINDOW_MATRIX_SHIFT * 4 < WINDOW_SIZE - 5

enum PointStatus {
    Ok,
    Unsatisfied(usize),
}

pub struct Chunker<'a> {
    buf: &'a [u8],
    position: usize,
    chunk_start: usize,
    has_cut: bool,
    sizes: SizeParams,
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
            position: sizes.min,
            chunk_start: 0,
            has_cut: false,
            sizes,
        }
    }

    fn is_point_satisfied(&self) -> PointStatus {
        // primary check, T<=x<M where T is WINDOW_SECONDARY_COUNT and M is WINDOW_COUNT
        for i in WINDOW_SECONDARY_COUNT..WINDOW_COUNT {
            if !self
                .is_window_qualified(&self.buf[self.position - i - WINDOW_SIZE..self.position - i])
            {
                // window is WINDOW_SIZE bytes long and moves to the left
                let leap = WINDOW_COUNT - i;
                return PointStatus::Unsatisfied(leap);
            }
        }

        //secondary check, 0<=x<T bytes
        for i in 0..WINDOW_SECONDARY_COUNT {
            if !self
                .is_window_qualified(&self.buf[self.position - i - WINDOW_SIZE..self.position - i])
            {
                let leap = WINDOW_COUNT - WINDOW_SECONDARY_COUNT - i;
                return PointStatus::Unsatisfied(leap);
            }
        }

        PointStatus::Ok
    }

    fn is_window_qualified(&self, window: &[u8]) -> bool {
        (0..5)
            .map(|index| window[WINDOW_SIZE - 1 - index * WINDOW_MATRIX_SHIFT]) // init array
            .enumerate()
            .map(|(index, byte)| EF_MATRIX[byte as usize][index]) // get elements from ef_matrix
            .fold(0u8, |acc, value| acc ^ value)
            != 0
    }
}

impl Iterator for Chunker<'_> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position == self.buf.len() {
            return if self.has_cut {
                None
            } else {
                self.has_cut = true;
                let chunk = Chunk::new(self.chunk_start, self.position - self.chunk_start);
                Some(chunk)
            };
        }

        while self.position < self.buf.len() {
            if self.position - self.chunk_start > self.sizes.max {
                let pos = self.chunk_start;
                let len = self.position - self.chunk_start;

                self.chunk_start = self.position;
                self.position += self.sizes.min;

                return Some(Chunk::new(pos, len));
            } else {
                match self.is_point_satisfied() {
                    PointStatus::Ok => {
                        let pos = self.chunk_start;
                        let len = self.position - self.chunk_start;

                        self.chunk_start = self.position;
                        self.position += self.sizes.min;

                        return Some(Chunk::new(pos, len));
                    }
                    PointStatus::Unsatisfied(leap) => {
                        self.position += leap;
                    }
                }
            }
        }

        self.position = self.buf.len();
        self.has_cut = true;
        Some(Chunk::new(
            self.chunk_start,
            self.position - self.chunk_start,
        ))
    }
}

const EF_MATRIX: [[u8; 5]; 256] = [
    [0, 0, 2, 0, 2],
    [1, 3, 3, 1, 3],
    [2, 0, 0, 3, 2],
    [3, 3, 3, 1, 2],
    [3, 3, 0, 1, 2],
    [1, 3, 3, 2, 1],
    [1, 3, 3, 1, 1],
    [0, 3, 3, 3, 0],
    [1, 3, 0, 2, 0],
    [2, 1, 3, 1, 1],
    [1, 0, 0, 1, 0],
    [0, 1, 1, 2, 3],
    [2, 3, 3, 0, 2],
    [1, 1, 3, 3, 1],
    [0, 0, 3, 2, 0],
    [3, 3, 1, 2, 2],
    [2, 0, 3, 3, 0],
    [2, 0, 1, 2, 1],
    [3, 3, 3, 1, 2],
    [0, 0, 2, 2, 2],
    [1, 1, 3, 3, 1],
    [1, 1, 3, 1, 3],
    [0, 1, 1, 3, 0],
    [3, 2, 3, 0, 3],
    [1, 3, 3, 1, 3],
    [3, 3, 2, 3, 1],
    [0, 3, 0, 1, 0],
    [1, 2, 3, 2, 0],
    [3, 1, 0, 2, 3],
    [2, 0, 1, 1, 0],
    [2, 2, 1, 2, 3],
    [0, 0, 1, 1, 1],
    [1, 0, 1, 2, 2],
    [1, 1, 0, 0, 3],
    [1, 3, 0, 0, 2],
    [2, 3, 0, 3, 1],
    [3, 3, 0, 1, 3],
    [2, 1, 3, 2, 1],
    [0, 1, 3, 2, 1],
    [0, 3, 2, 0, 2],
    [0, 0, 0, 3, 1],
    [3, 3, 0, 1, 3],
    [3, 3, 0, 1, 2],
    [3, 1, 0, 1, 0],
    [2, 3, 1, 0, 0],
    [1, 2, 2, 2, 1],
    [2, 3, 1, 0, 2],
    [3, 1, 0, 3, 0],
    [3, 0, 3, 1, 2],
    [3, 2, 3, 3, 3],
    [2, 2, 0, 1, 2],
    [2, 3, 0, 2, 2],
    [3, 1, 3, 2, 1],
    [1, 2, 3, 0, 0],
    [0, 3, 3, 0, 0],
    [1, 0, 3, 2, 1],
    [0, 2, 0, 1, 1],
    [2, 3, 2, 3, 2],
    [0, 1, 2, 1, 3],
    [0, 3, 0, 1, 3],
    [1, 1, 1, 3, 2],
    [1, 3, 2, 0, 2],
    [0, 0, 1, 1, 1],
    [2, 3, 2, 3, 1],
    [2, 3, 2, 0, 2],
    [2, 1, 3, 1, 2],
    [3, 1, 0, 0, 0],
    [1, 3, 1, 2, 1],
    [2, 3, 0, 1, 1],
    [3, 3, 3, 2, 3],
    [3, 0, 1, 3, 0],
    [0, 0, 3, 1, 1],
    [2, 2, 3, 3, 2],
    [3, 3, 1, 0, 0],
    [2, 1, 1, 3, 1],
    [3, 1, 0, 1, 0],
    [0, 0, 1, 2, 3],
    [0, 1, 3, 0, 1],
    [2, 3, 1, 0, 0],
    [2, 1, 0, 2, 1],
    [2, 0, 3, 1, 1],
    [0, 1, 3, 1, 2],
    [3, 2, 2, 2, 3],
    [1, 1, 2, 1, 3],
    [0, 0, 2, 3, 3],
    [1, 3, 3, 3, 0],
    [3, 0, 0, 0, 2],
    [2, 3, 3, 1, 1],
    [3, 1, 2, 3, 1],
    [0, 2, 0, 0, 3],
    [0, 3, 2, 2, 0],
    [3, 3, 2, 0, 1],
    [0, 3, 1, 0, 1],
    [1, 1, 0, 1, 0],
    [3, 0, 3, 3, 3],
    [1, 3, 1, 0, 0],
    [2, 3, 0, 0, 2],
    [1, 3, 3, 3, 3],
    [0, 2, 0, 0, 2],
    [0, 3, 1, 1, 1],
    [2, 3, 3, 2, 2],
    [1, 3, 2, 2, 1],
    [2, 1, 1, 2, 3],
    [0, 2, 1, 1, 0],
    [2, 1, 2, 1, 2],
    [1, 3, 2, 1, 0],
    [1, 1, 1, 0, 2],
    [2, 1, 0, 2, 3],
    [0, 1, 3, 2, 3],
    [0, 3, 1, 3, 2],
    [1, 0, 1, 2, 3],
    [2, 3, 0, 0, 0],
    [2, 3, 3, 2, 0],
    [0, 1, 1, 3, 2],
    [2, 0, 1, 2, 0],
    [1, 0, 3, 1, 0],
    [1, 1, 1, 2, 1],
    [3, 3, 1, 1, 2],
    [2, 2, 1, 3, 0],
    [3, 0, 2, 1, 1],
    [1, 0, 3, 1, 0],
    [0, 2, 2, 1, 0],
    [0, 0, 3, 1, 3],
    [3, 1, 3, 3, 2],
    [1, 3, 3, 2, 2],
    [0, 3, 0, 1, 0],
    [2, 0, 2, 1, 2],
    [0, 2, 2, 1, 1],
    [3, 1, 1, 2, 2],
    [1, 3, 1, 2, 1],
    [3, 0, 3, 2, 3],
    [2, 0, 0, 1, 1],
    [0, 2, 0, 0, 1],
    [3, 3, 0, 2, 0],
    [3, 1, 1, 2, 3],
    [2, 3, 0, 2, 3],
    [0, 3, 1, 2, 2],
    [1, 1, 2, 0, 3],
    [0, 0, 2, 2, 1],
    [2, 2, 2, 1, 2],
    [2, 3, 0, 2, 3],
    [1, 3, 2, 1, 3],
    [3, 2, 2, 0, 1],
    [1, 0, 0, 1, 3],
    [1, 0, 3, 3, 3],
    [2, 3, 2, 1, 0],
    [3, 0, 2, 0, 1],
    [3, 2, 0, 1, 0],
    [1, 2, 3, 1, 0],
    [2, 2, 2, 3, 1],
    [2, 0, 1, 2, 3],
    [1, 2, 1, 2, 1],
    [3, 1, 2, 2, 3],
    [1, 2, 2, 1, 0],
    [2, 0, 1, 1, 2],
    [1, 0, 0, 1, 1],
    [3, 0, 2, 2, 2],
    [3, 1, 3, 3, 1],
    [2, 0, 0, 0, 0],
    [1, 0, 3, 3, 1],
    [2, 0, 2, 3, 3],
    [0, 3, 0, 0, 0],
    [2, 2, 3, 2, 3],
    [3, 0, 2, 3, 2],
    [0, 0, 1, 3, 2],
    [3, 0, 1, 1, 3],
    [3, 1, 3, 3, 0],
    [0, 2, 1, 0, 2],
    [1, 0, 0, 2, 2],
    [0, 3, 3, 3, 1],
    [2, 0, 0, 0, 3],
    [3, 3, 1, 0, 0],
    [2, 2, 1, 2, 0],
    [0, 1, 1, 1, 0],
    [3, 2, 0, 2, 1],
    [1, 3, 0, 2, 2],
    [1, 2, 3, 1, 2],
    [1, 0, 2, 3, 3],
    [3, 2, 0, 3, 2],
    [3, 3, 2, 1, 0],
    [0, 2, 3, 2, 3],
    [1, 2, 2, 0, 2],
    [0, 0, 2, 3, 3],
    [1, 1, 0, 0, 1],
    [3, 3, 0, 2, 2],
    [0, 3, 2, 0, 3],
    [0, 0, 0, 1, 0],
    [1, 0, 3, 2, 2],
    [2, 0, 2, 1, 2],
    [0, 2, 3, 3, 3],
    [1, 2, 0, 2, 1],
    [1, 0, 1, 3, 1],
    [1, 0, 1, 0, 2],
    [3, 3, 2, 2, 2],
    [2, 0, 1, 3, 1],
    [2, 2, 2, 0, 1],
    [3, 0, 3, 2, 0],
    [3, 2, 1, 2, 0],
    [1, 0, 1, 0, 1],
    [3, 1, 3, 2, 2],
    [2, 3, 0, 1, 2],
    [3, 0, 0, 3, 3],
    [2, 1, 0, 3, 3],
    [0, 2, 0, 1, 2],
    [1, 0, 3, 1, 1],
    [1, 1, 3, 2, 1],
    [0, 1, 0, 0, 0],
    [0, 3, 0, 2, 1],
    [0, 2, 3, 0, 3],
    [1, 0, 2, 3, 1],
    [2, 1, 1, 1, 2],
    [1, 0, 2, 3, 3],
    [0, 2, 3, 2, 3],
    [0, 0, 3, 2, 1],
    [0, 0, 3, 2, 0],
    [3, 3, 3, 0, 2],
    [3, 0, 1, 3, 1],
    [3, 2, 0, 1, 2],
    [1, 2, 0, 1, 2],
    [0, 0, 3, 2, 0],
    [1, 0, 3, 0, 2],
    [2, 0, 3, 3, 1],
    [2, 2, 3, 3, 0],
    [2, 3, 2, 1, 1],
    [3, 3, 2, 2, 2],
    [1, 1, 2, 1, 0],
    [1, 3, 2, 2, 3],
    [0, 2, 3, 1, 0],
    [2, 1, 0, 1, 3],
    [3, 0, 3, 2, 3],
    [0, 0, 1, 0, 2],
    [2, 0, 0, 2, 0],
    [0, 1, 0, 3, 0],
    [3, 2, 2, 0, 3],
    [2, 2, 0, 2, 0],
    [2, 2, 0, 0, 2],
    [3, 3, 1, 1, 1],
    [0, 0, 0, 2, 1],
    [1, 3, 2, 1, 2],
    [1, 3, 0, 0, 3],
    [0, 0, 2, 1, 1],
    [3, 3, 0, 1, 3],
    [2, 2, 0, 0, 2],
    [1, 0, 0, 3, 1],
    [3, 2, 2, 1, 0],
    [2, 3, 3, 2, 3],
    [1, 2, 0, 2, 2],
    [2, 0, 3, 1, 3],
    [3, 0, 0, 0, 3],
    [2, 0, 0, 2, 2],
    [2, 0, 0, 1, 2],
    [0, 0, 3, 2, 1],
    [0, 0, 0, 2, 1],
    [1, 3, 3, 0, 1],
    [2, 0, 0, 2, 0],
    [3, 3, 1, 3, 1],
];
