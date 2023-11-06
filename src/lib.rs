pub mod leap_based;
pub mod quick;
pub mod ultra;
pub mod supercdc;

#[derive(Debug)]
pub struct Chunk {
    // TODO: not pub
    pub pos: usize,
    pub len: usize,
}

impl Chunk {
    pub fn new(pos: usize, len: usize) -> Self {
        Chunk { pos, len }
    }
}
