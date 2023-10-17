pub mod leap_based;
pub mod quick;
pub mod ultra;

#[derive(Debug)]
pub struct Chunk {
    // TODO: not pub
    pub pos: usize,
    pub len: usize,
}

impl Chunk {
    fn new(pos: usize, len: usize) -> Self {
        Chunk { pos, len }
    }
}
