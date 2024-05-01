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
