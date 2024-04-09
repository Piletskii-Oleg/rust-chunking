use crate::Chunk;
use rand::prelude::ThreadRng;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};

const MIN_CHUNK_SIZE: usize = 1024 * 8;
const MAX_CHUNK_SIZE: usize = 1024 * 16;

const WINDOW_PRIMARY_COUNT: usize = 22;
const WINDOW_SECONDARY_COUNT: usize = 2;
const WINDOW_COUNT: usize = WINDOW_PRIMARY_COUNT + WINDOW_SECONDARY_COUNT;

const WINDOW_SIZE: usize = 180;
const WINDOW_MATRIX_SHIFT: usize = 42; // WINDOW_MATRIX_SHIFT * 4 < WINDOW_SIZE - 5
const MATRIX_WIDTH: usize = 8;
const MATRIX_HEIGHT: usize = 255;

static EF_MATRIX: Vec<Vec<u8>> = Chunker::create_ef_matrix();

enum PointStatus {
    Ok,
    Unsatisfied(usize),
}

pub struct Chunker<'a> {
    buf: &'a [u8],
    position: usize,
    chunk_start: usize,
    has_cut: bool,
}

impl<'a> Chunker<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Chunker {
            buf,
            position: MIN_CHUNK_SIZE,
            chunk_start: 0,
            has_cut: false,
        }
    }

    fn create_ef_matrix() -> Vec<Vec<u8>> {
        let base_matrix = (0..=255)
            .map(|index| vec![index; 5])
            .collect::<Vec<Vec<u8>>>(); // 256x5 matrix that looks like ((0,0,0,0,0), (1,1,1,1,1)..)

        let matrix_h = Chunker::generate_matrix();
        let matrix_g = Chunker::generate_matrix();

        let e_matrix = Chunker::transform_base_matrix(&base_matrix, &matrix_h);
        let f_matrix = Chunker::transform_base_matrix(&base_matrix, &matrix_g);

        let ef_matrix = e_matrix
            .iter()
            .zip(f_matrix.iter())
            .map(Chunker::concatenate_bits_in_rows)
            .collect();
        ef_matrix
    }

    fn transform_base_matrix(
        base_matrix: &[Vec<u8>],
        additional_matrix: &[Vec<f64>],
    ) -> Vec<Vec<bool>> {
        base_matrix
            .iter()
            .map(|row| Chunker::transform_byte_row(row[0], additional_matrix))
            .collect::<Vec<Vec<bool>>>()
    }

    fn concatenate_bits_in_rows((row_x, row_y): (&Vec<bool>, &Vec<bool>)) -> Vec<u8> {
        row_x
            .iter()
            .zip(row_y.iter())
            .map(Chunker::concatenate_bits)
            .collect()
    }

    fn concatenate_bits((x, y): (&bool, &bool)) -> u8 {
        match (*x, *y) {
            (true, true) => 3,
            (true, false) => 2,
            (false, true) => 1,
            (false, false) => 0,
        }
    }

    fn transform_byte_row(byte: u8, matrix: &[Vec<f64>]) -> Vec<bool> {
        let mut new_row = [0u8; 5];
        (0..255)
            .map(|index| Chunker::multiply_rows(byte, &matrix[index]))
            .enumerate()
            .for_each(|(index, value)| {
                if value > 0.0 {
                    new_row[index / 51] += 1;
                }
            });

        new_row
            .iter()
            .map(|&number| number % 2 != 0)
            .collect::<Vec<bool>>()
    }

    fn multiply_rows(byte: u8, numbers: &[f64]) -> f64 {
        numbers
            .iter()
            .enumerate()
            .map(|(index, number)| {
                if (byte >> index) & 1 == 1 {
                    *number
                } else {
                    -(*number)
                }
            })
            .sum()
    }

    fn generate_matrix() -> Vec<Vec<f64>> {
        let normal = Normal::new(0.0, 1.0).unwrap();
        let mut rng = thread_rng();

        (0..MATRIX_HEIGHT)
            .map(|_| Chunker::generate_row(&normal, &mut rng))
            .collect()
    }

    fn generate_row(normal: &Normal<f64>, rng: &mut ThreadRng) -> Vec<f64> {
        (0..MATRIX_WIDTH).map(|_| normal.sample(rng)).collect()
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
                let chunk = Chunk::new(
                    self.chunk_start,
                    self.position - self.chunk_start,
                );
                Some(chunk)
            }
        }

        while self.position < self.buf.len() {
            if self.position - self.chunk_start > MAX_CHUNK_SIZE {
                let pos = self.chunk_start;
                let len = self.position - self.chunk_start;

                self.chunk_start = self.position;
                self.position += MIN_CHUNK_SIZE;

                return Some(Chunk::new(pos, len));
            } else {
                match self.is_point_satisfied() {
                    PointStatus::Ok => {
                        let pos = self.chunk_start;
                        let len = self.position - self.chunk_start;

                        self.chunk_start = self.position;
                        self.position += MIN_CHUNK_SIZE;

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
