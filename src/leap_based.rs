#![allow(dead_code)]

use std::ops::Add;
use std::time::{Duration, Instant};
use rand::prelude::ThreadRng;
use rand::thread_rng;
use rand_distr::{Distribution, Normal};

const MIN_CHUNK_SIZE: usize = 4096; // 4 KB
const MAX_CHUNK_SIZE: usize = 12288; // 12 KB

const WINDOW_PRIMARY_COUNT: usize = 22;
const WINDOW_SECONDARY_COUNT: usize = 2;
const WINDOW_COUNT: usize = WINDOW_PRIMARY_COUNT + WINDOW_SECONDARY_COUNT;

const WINDOW_SIZE: usize = 128;
const WINDOW_MATRIX_SHIFT: usize = 24; // WINDOW_MATRIX_SHIFT * 4 < WINDOW_SIZE - 5
const MATRIX_WIDTH: usize = 8;
const MATRIX_HEIGHT: usize = 255;

enum PointStatus {
    Ok,
    Unsatisfied(usize),
}

#[derive(Debug)]
pub struct Chunk {
    pub(super) pos: usize,
    pub(super) len: usize,
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

        (0..MATRIX_HEIGHT)
            .map(|_| Chunker::generate_row(&normal, &mut rng))
            .collect()
    }

    fn generate_row(normal: &Normal<f64>, rng: &mut ThreadRng) -> Vec<f64> {
        (0..MATRIX_WIDTH)
            .map(|_| normal.sample(rng))
            .collect()
    }

    fn is_point_satisfied(&self, index: usize, data: &[u8]) -> PointStatus {
        // primary check, T+1<=x<M where T is WINDOW_SECONDARY_COUNT and M is WINDOW_COUNT
        for i in WINDOW_SECONDARY_COUNT..WINDOW_COUNT {
            if !self.is_window_qualified(&data[index - i - WINDOW_SIZE..index - i]) { // window is WINDOW_SIZE bytes long and moves to the left
                let leap = WINDOW_COUNT - i;
                return PointStatus::Unsatisfied(leap);
            }
        }

        //secondary check, 0<=x<T bytes
        for i in 0..WINDOW_SECONDARY_COUNT {
            if !self.is_window_qualified(&data[index - i - WINDOW_SIZE..index - i]) {
                let leap = WINDOW_COUNT - WINDOW_SECONDARY_COUNT - i;
                return PointStatus::Unsatisfied(leap);
            }
        }

        PointStatus::Ok
    }

    fn is_window_qualified(&self, window: &[u8]) -> bool {
        let input = (0..5)
            .map(|index| window[WINDOW_SIZE - 1 - index * WINDOW_MATRIX_SHIFT]) // init array
            .map(byte_to_bits) // transform bytes to bit arrays
            .collect();

        let positive_one = self.transform_input(&input, &self.matrix_h);
        let positive_two = self.transform_input(&input, &self.matrix_g);

        positive_one % 2 == 1 || positive_two % 2 == 1
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
            .map(|sign| if *sign {1.0} else {-1.0})
            .zip(row_2.iter())
            .map(|(sign, number)| sign * number)
            .collect()
    }
}

fn byte_to_bits(number: u8) -> Vec<bool> {
    (0..8)
        .rev()
        .map(|n| if (number >> n) & 1 == 1 { true } else { false })
        .collect()
}

pub fn generate_chunks(data: &[u8]) -> Vec<Chunk> {
    let mut chunks = vec![];
    let chunker = Chunker::new();

    let mut chunk_start = 0;
    let mut index = MIN_CHUNK_SIZE;

    let mut total = Duration::from_micros(0);
    let mut times = vec![];

    while index < data.len() {
        let now = Instant::now();
        if index - chunk_start > MAX_CHUNK_SIZE {
            chunks.push(Chunk::new(chunk_start, index - chunk_start));
            chunk_start = index;
            index += MIN_CHUNK_SIZE;
            println!("Added chunk: {:?}", chunks.last());
            if chunks.len() > 1 {
                println!("Chunks are aligned: {}", chunks[chunks.len() - 2].pos + chunks[chunks.len() - 2].len == chunks.last().unwrap().pos)
            }
        } else {
            match chunker.is_point_satisfied(index, data) {
                PointStatus::Ok => {
                    chunks.push(Chunk::new(chunk_start, index - chunk_start));
                    chunk_start = index;
                    index += MIN_CHUNK_SIZE;
                    println!("Added chunk: {:?}", chunks.last());
                    if chunks.len() > 1 {
                        println!("Chunks are aligned: {}", chunks[chunks.len() - 2].pos + chunks[chunks.len() - 2].len == chunks.last().unwrap().pos)
                    }
                }
                PointStatus::Unsatisfied(leap) => {
                    index += leap;
                },
            };
        }
        total = total.add(now.elapsed());
        times.push(0);
    }

    if index >= data.len() {
        index = data.len();
        chunks.push(Chunk::new(chunk_start, index - chunk_start));
        println!("Added chunk: {:?}", chunks.last());
        println!("Last chunk is aligned: {}", chunks[chunks.len() - 1].pos + chunks[chunks.len() - 1].len == 660000)
    }

    println!("{} ms average for an iteration point", total.as_micros() / times.len() as u128);
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
