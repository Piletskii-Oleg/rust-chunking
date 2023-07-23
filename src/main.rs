use std::env;
use std::time::Instant;
use crate::leap_based::generate_chunks;

mod leap_based;

fn main() {
    let to_chunk: Vec<u8> = generate_data(660000);
    println!("Generated data. Calculating chunks...");

    let now = Instant::now();

    let chunks = generate_chunks(&to_chunk);

    println!("{:?}", chunks);
    println!("{:?}", now.elapsed());
    let lens: usize = chunks.iter().map(|chunk| chunk.len).sum();
    println!("Average len: {}", lens / chunks.len());
}

fn generate_data(size: usize) -> Vec<u8> {
    (0..size)
        .map(|_| rand::random::<u8>())
        .collect()
}
