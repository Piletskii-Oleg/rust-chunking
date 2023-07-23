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

pub struct Chunker {
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
        let normal = Normal::new(0.0, 1.0).unwrap();
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
        // primary check, T<=x<M where T is WINDOW_SECONDARY_COUNT and M is WINDOW_COUNT
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
            .collect::<Vec<u8>>();

        self.transform_input(&input, &self.matrix_h) % 2 == 1 ||
            self.transform_input(&input, &self.matrix_g) % 2 == 1
    }

    fn transform_input(&self, input: &[u8], matrix: &[Vec<f64>]) -> usize {
        matrix.iter()
            .map(|matrix_row| Chunker::multiply_rows(input, matrix_row))
            .filter(|number| *number > 0.0)
            .count()
    }

    fn multiply_rows(bytes: &[u8], numbers: &[f64]) -> f64 {
        bytes
            .iter()
            .zip(numbers.iter())
            .enumerate()
            .map(|(index, (byte, number))| if (*byte >> index) & 1 == 1 {*number} else {-(*number)})
            .sum()
    }
}

pub fn generate_chunks(chunker: &Chunker, data: &[u8]) -> Vec<Chunk> {
    let mut chunks = vec![];

    let mut chunk_start = 0;
    let mut index = MIN_CHUNK_SIZE;

    while index < data.len() {
        if index - chunk_start > MAX_CHUNK_SIZE {
            chunks.push(Chunk::new(chunk_start, index - chunk_start));
            chunk_start = index;
            index += MIN_CHUNK_SIZE;
            if chunks.len() > 1 {
                assert_eq!(chunks[chunks.len() - 2].pos + chunks[chunks.len() - 2].len, chunks.last().unwrap().pos);
            }
        } else {
            match chunker.is_point_satisfied(index, data) {
                PointStatus::Ok => {
                    chunks.push(Chunk::new(chunk_start, index - chunk_start));
                    chunk_start = index;
                    index += MIN_CHUNK_SIZE;
                    if chunks.len() > 1 {
                        assert_eq!(chunks[chunks.len() - 2].pos + chunks[chunks.len() - 2].len, chunks.last().unwrap().pos);
                    }
                }
                PointStatus::Unsatisfied(leap) => {
                    index += leap;
                },
            };
        }
    }

    if index >= data.len() {
        index = data.len();
        chunks.push(Chunk::new(chunk_start, index - chunk_start));
        assert_eq!(chunks[chunks.len() - 1].pos + chunks[chunks.len() - 1].len, 6600000)
    }

    chunks
}
