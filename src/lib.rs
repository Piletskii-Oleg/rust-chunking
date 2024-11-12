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
    pub min: usize,
    pub avg: usize,
    pub max: usize,
}

impl SizeParams {
    pub fn new(min: usize, avg: usize, max: usize) -> Self {
        Self { min, avg, max }
    }

    pub fn leap_default() -> Self {
        leap_based::Chunker::default_sizes()
    }

    pub fn rabin_default() -> Self {
        rabin::Chunker::default_sizes()
    }

    pub fn super_default() -> Self {
        supercdc::Chunker::default_sizes()
    }

    pub fn ultra_default() -> Self {
        ultra::Chunker::default_sizes()
    }
}
