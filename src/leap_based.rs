#![allow(dead_code)]

use rand::thread_rng;
use rand_distr::Distribution;

const MIN_CHUNK_SIZE: usize = 4096; // 4 KB
const MAX_CHUNK_SIZE: usize = 12288; // 12 KB

const WINDOW_SIZE: usize = 128;
const WINDOW_COUNT: usize = 24;
const WINDOW_EXTRA_COUNT: usize = 2;
const WINDOW_MATRIX_SHIFT: usize = 26; // WINDOW_MATRIX_SHIFT * 4 < WINDOW_SIZE - 5

const MATRIX_WIDTH: usize = 8;
const MATRIX_HEIGHT: usize = 255;

enum PointStatus {
    Ok,
    Unsatisfied(usize),
}

struct Chunk {
    pos: usize,
    len: usize,
}

impl Chunk {
    fn new(pos: usize, len: usize) -> Self {
        Chunk { pos, len }
    }
}

struct Chunker {
    matrix_h: Vec<Vec<f64>>,
    matrix_g: Vec<Vec<f64>>,
}

impl Chunker {
    pub fn new() -> Self {
        Chunker {
            matrix_h: Chunker::generate_matrix(),
            matrix_g: Chunker::generate_matrix(),
        }
    }

    fn generate_matrix() -> Vec<Vec<f64>> {
        let normal = rand_distr::Normal::new(0.0, 1.0).unwrap();
        let mut rng = thread_rng();
        vec![vec![normal.sample(&mut rng); MATRIX_WIDTH]; MATRIX_HEIGHT]
    }

    fn is_point_satisfied(&self, index: usize, data: &[u8]) -> PointStatus {
        for i in 0..WINDOW_COUNT {
            if !self.is_window_qualified(&data[index - i - WINDOW_SIZE..=index - i]) {
                let leap = WINDOW_COUNT - i;
                return PointStatus::Unsatisfied(leap);
            }
        }

        PointStatus::Ok
    }

    fn is_window_qualified(&self, window: &[u8]) -> bool {
        let input = (0..5)
            .map(|index| window[WINDOW_SIZE - index * WINDOW_MATRIX_SHIFT]) // get elements
            .map(byte_to_bits)
            .collect();

        let positive_one = self.transform_input(&input, &self.matrix_h);
        let positive_two = self.transform_input(&input, &self.matrix_g);

        return positive_one % 2 == 1 || positive_two % 2 == 1
    }

    fn transform_input(&self, input: &Vec<Vec<bool>>, matrix: &Vec<Vec<f64>>) -> usize {
        matrix.iter().enumerate()
            .map(|(index, matrix_row)| Chunker::multiply_rows(&input[index % 5], matrix_row))
            .map(|row| row.iter().sum())
            .filter(|number: &f64| *number > 0.0)
            .count()
    }

    fn multiply_rows(row_1: &[bool], row_2: &[f64]) -> Vec<f64> {
        row_1
            .iter()
            .zip(row_2.iter())
            .map(|(sign, number)| if *sign { *number } else { -(*number) })
            .collect()
    }
}

fn byte_to_bits(number: u8) -> Vec<bool> {
    (0..8)
        .rev()
        .map(|n| if (number >> n) & 1 == 1 { true } else { false })
        .collect()
}

fn generate_chunks(data: &[u8]) -> Vec<Chunk> {
    let mut chunks = vec![];
    let chunker = Chunker::new();

    let mut chunk_start = 0;
    let mut index = MIN_CHUNK_SIZE;

    while index < data.len() {
        if index - chunk_start > MAX_CHUNK_SIZE {
            chunks.push(Chunk::new(chunk_start, index - chunk_start));
            chunk_start = index;
            index += MIN_CHUNK_SIZE;
        }

        match chunker.is_point_satisfied(index, data) {
            PointStatus::Ok => {
                chunks.push(Chunk::new(chunk_start, index - chunk_start));
                chunk_start = index;
                index += MIN_CHUNK_SIZE;
            }
            PointStatus::Unsatisfied(leap) => index += leap,
        };
    }

    chunks
}

#[cfg(test)]
mod tests {
    use crate::leap_based::*;

    fn num_to_bool(value: &str) -> Vec<bool> {
        value.chars().map(|x| x == '1').collect()
    }

    #[test]
    fn byte_to_bits_test() {
        assert_eq!(byte_to_bits(194), num_to_bool("11000010"));
        assert_eq!(byte_to_bits(53), num_to_bool("00110101"))
    }

    #[test]
    fn multiply_rows_test() {
        let row_1 = [true, false, false, true];
        let row_2 = [3.2, 8.8, -2.1, -7.4];
        assert_eq!(Chunker::multiply_rows(&row_1, &row_2), [3.2, -8.8, 2.1, -7.4]);
    }
}
