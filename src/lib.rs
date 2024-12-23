use std::fmt::{Display, Formatter};

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
    /// Creates a new instance of `SizeParams` struct.
    ///
    /// Panics if not (min <= avg && avg <= max && min <= max).
    pub fn new(min: usize, avg: usize, max: usize) -> Self {
        assert!(min <= avg && avg <= max && min <= max);

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

impl Display for SizeParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}-{}", self.min, self.avg, self.max)
    }
}
