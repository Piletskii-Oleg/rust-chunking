use std::time::Instant;
use crate::leap_based::{Chunker, generate_chunks};

mod leap_based;

fn main() {
    let now = Instant::now();
    let to_chunk: Vec<u8> = generate_data(6600000);
    println!("Generated data in {:?}. Calculating chunks...", now.elapsed());

    let chunker = Chunker::new();

    let now = Instant::now();
    let chunks = generate_chunks(&chunker, &to_chunk);
    println!("Calculated in {:?}", now.elapsed());

    let lens: usize = chunks.iter().map(|chunk| chunk.len).sum();
    println!("Average len: {} bytes", lens / chunks.len());
}

fn generate_data(size: usize) -> Vec<u8> {
    (0..size)
        .map(|_| rand::random::<u8>())
        .collect()
}
