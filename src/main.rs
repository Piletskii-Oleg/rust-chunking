use std::collections::HashMap;
use std::time::Instant;
use chunking::leap_based::{Chunker, generate_chunks};

fn main() {
    const DATA_SIZE: usize = 1024 * 1024 * 1024 * 2;
    let now = Instant::now();
    let to_chunk: Vec<u8> = generate_data(DATA_SIZE);
    println!("Generated data ({} bytes) in {:?}. Calculating chunks...", to_chunk.len(), now.elapsed());

    let chunker = Chunker::new();

    let now = Instant::now();
    let chunks = generate_chunks(&chunker, &to_chunk);
    let time = now.elapsed();
    println!("Calculated in {:?}", time);

    let lens = chunks.iter().map(|chunk| chunk.len).collect::<Vec<usize>>();
    println!("Average len: {} bytes", lens.iter().sum::<usize>() / chunks.len());
    println!("Median: {} bytes", lens[lens.len() / 2]);
    println!("Mode: {} bytes", mode(&lens));

    println!("Speed: {} MB/s", DATA_SIZE / time.as_millis() as usize / 1024)
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
