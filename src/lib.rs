pub mod leap_based;
pub mod ultra;
pub mod rabin;
pub mod supercdc;

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
