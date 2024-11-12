pub mod leap_based;
pub mod rabin;
pub mod supercdc;
pub mod ultra;

#[derive(Debug)]
pub struct Chunk {
    pub pos: usize,
    pub len: usize,
}

impl Chunk {
    fn new(pos: usize, len: usize) -> Self {
        Chunk { pos, len }
    }
}

/// Struct containing size parameters for chunkers:
/// min, average and max size of chunks.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SizeParams {
    pub min_size: usize,
    pub avg_size: usize,
    pub max_size: usize,
}

impl SizeParams {
    pub fn new(min_size: usize, avg_size: usize, max_size: usize) -> Self {
        Self {
            min_size,
            avg_size,
            max_size,
        }
    }
}

impl Default for SizeParams {
    fn default() -> Self {
        Self {
            min_size: 2048,
            avg_size: 4096,
            max_size: 8192,
        }
    }
}