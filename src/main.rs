use std::collections::HashMap;
use std::time::Instant;
use crate::leap_based::{Chunker, generate_chunks};

mod leap_based;
mod quick;

fn main() {
    let now = Instant::now();
    let to_chunk: Vec<u8> = generate_data(6000000);
    println!("Generated data ({} bytes) in {:?}. Calculating chunks...", to_chunk.len(), now.elapsed());

    let chunker = Chunker::new();

    let now = Instant::now();
    let chunks = generate_chunks(&chunker, &to_chunk);
    println!("Calculated in {:?}", now.elapsed());

    let lens = chunks.iter().map(|chunk| chunk.len).collect::<Vec<usize>>();
    println!("Average len: {} bytes", lens.iter().sum::<usize>() / chunks.len());
    println!("Median: {} bytes", lens[lens.len() / 2]);
    println!("Mode: {} bytes", mode(&lens));
}

fn generate_data(size: usize) -> Vec<u8> {
    (0..size)
        .map(|_| rand::random::<u8>())
        .collect()
}

fn mode(numbers: &[usize]) -> usize {
    let mut occurrences = HashMap::new();

    for &value in numbers {
        *occurrences.entry(value).or_insert(0) += 1;
    }

    occurrences
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(val, _)| val)
        .expect("Cannot compute the mode of zero numbers")
}
