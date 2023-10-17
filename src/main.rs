use chunking::ultra;
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;
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

    let buf = std::fs::read("/home/olegp/projects/rust-chunking/ubuntu.iso").unwrap();

    let mut chunker = ultra::Chunker::new();

    let now = Instant::now();
    let chunks = chunker.generate_chunks(&buf);
    let time = now.elapsed();
    println!(
        "Chunked file with size {}MB in {:?}",
        buf.len() / 1024 / 1024,
        time
    );

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

    let total_len = chunks.iter().map(|chunk| chunk.len).sum::<usize>();
    assert_eq!(total_len, buf.len());

    let chunks_len = chunks.len();
    let chunks_map: HashMap<_, usize> = HashMap::from_iter(chunks.into_iter().map(|chunk| {
        let hash = Sha3_256::digest(&buf[chunk.pos..chunk.pos + chunk.len]);
        let mut res = vec![0u8; hash.len()];
        res.copy_from_slice(&hash);
        (res, chunk.len)
    }));
    println!(
        "Chunk ratio (unique / all): {} / {} = {:.3}",
        chunks_map.len(),
        chunks_len,
        chunks_map.len() as f64 / chunks_len as f64
    );
    println!(
        "Data size ratio: {} / {} = {:.3}",
        chunks_map.iter().map(|(a, &b)| b).sum::<usize>(),
        buf.len(),
        chunks_map.iter().map(|(a, &b)| b).sum::<usize>() as f64 / buf.len() as f64
    );
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
