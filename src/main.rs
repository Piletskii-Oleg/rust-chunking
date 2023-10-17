use chunking::leap_based::{generate_chunks, Chunker};
use chunking::{Chunk, ultra};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::time::Instant;

fn main() {
    test_chunker();
}

fn test_chunker() {
    // const DATA_SIZE: usize = 1024 * 1024 * 100 * 2;
    // let now = Instant::now();
    // let to_chunk: Vec<u8> = generate_data(DATA_SIZE);
    // println!(
    //     "Generated data ({} bytes) in {:?}. Calculating chunks...",
    //     to_chunk.len(),
    //     now.elapsed()
    // );

    let buf = std::fs::read("discord-bot").unwrap();

    let mut chunker = ultra::Chunker::new();

    let now = Instant::now();
    let chunks = chunker.generate_chunks(&buf);
    let time = now.elapsed();
    println!("Chunked file with size {}MB in {:?}", buf.len() / 1024 / 1024, time);

    let lens = chunks.iter().map(|chunk| chunk.len).collect::<Vec<usize>>();
    println!(
        "Average len: {} bytes",
        lens.iter().sum::<usize>() / chunks.len()
    );
    println!("Median: {} bytes", lens[lens.len() / 2]);
    println!("Mode: {} bytes", mode(&lens));

    println!(
        "Speed: {} MB/s",
        buf.len() / time.as_millis() as usize / 1024
    );

    let chunks_len = chunks.len();
    let chunks_map: HashMap<Chunk, usize> = HashMap::from_iter(
        chunks.into_iter().map(|chunk| { (chunk.clone(), chunk.len)})
    );
    println!("Chunk ratio: {} / {} = {}", chunks_map.len(), chunks_len, chunks_map.len() / chunks_len);
    println!("Data size ratio: {} / {} = {}",
    chunks_map.iter().map(|(a, &b)| b).sum::<usize>(),
    buf.len(),
             chunks_map.iter().map(|(a, &b)| b).sum::<usize>() / buf.len());
}

fn generate_data(size: usize) -> Vec<u8> {
    (0..size).map(|_| rand::random::<u8>()).collect()
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
