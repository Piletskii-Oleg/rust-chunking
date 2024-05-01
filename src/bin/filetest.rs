use chunking::{leap_based, ultra, rabin, Chunk};
use clap::Parser;
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;
use std::time::{Duration, Instant};

fn main() {
    let cli = Cli::parse();

    const DEFAULT_PATH: &str = "ubuntu.iso";
    let buf = if let Some(size) = cli.input.generate {
        generate_data(size)
    } else {
        let path = if let Some(path) = cli.input.path {
            path
        } else {
            DEFAULT_PATH.to_string()
        };
        std::fs::read(path).expect("Unable to read file:")
    };
    let (chunks, time) = match cli.algorithm {
        Algorithm::Ultra => chunk_file(ultra::Chunker::new(&buf)),
        Algorithm::Leap => chunk_file(leap_based::Chunker::new(&buf)),
        Algorithm::Rabin => chunk_file(rabin::Chunker::new(&buf))
    };

    let total_len = chunks.iter().map(|chunk| chunk.len).sum::<usize>();
    assert_eq!(total_len, buf.len());

    print_info(&buf, &chunks, time);

    if cli.dedup_ratio {
        dedup_info(&buf, chunks);
    }
}

fn print_info(buf: &[u8], chunks: &[Chunk], time: Duration) {
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
}

fn chunk_file(chunker: impl Iterator<Item = Chunk>) -> (Vec<Chunk>, Duration) {
    let now = Instant::now();
    let mut chunks = Vec::new();
    for chunk in chunker {
        chunks.push(chunk); // TODO: allocation times are counted
    }
    let time = now.elapsed();
    (chunks, time)
}

fn dedup_info(buf: &[u8], chunks: Vec<Chunk>) {
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
        chunks_map.iter().map(|(_, &b)| b).sum::<usize>(),
        buf.len(),
        chunks_map.iter().map(|(_, &b)| b).sum::<usize>() as f64 / buf.len() as f64
    );
}

fn generate_data(size: usize) -> Vec<u8> {
    let bytes = size * 1024 * 1024;
    (0..bytes).map(|_| rand::random::<u8>()).collect()
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

#[derive(clap::Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(flatten)]
    pub input: Input,

    /// Calculate deduplication ratio
    #[arg(short, long)]
    pub dedup_ratio: bool,

    /// What algorithm to use on the file
    #[arg(value_enum)]
    pub algorithm: Algorithm,
}

#[derive(clap::Args)]
#[group(multiple = false)]
pub struct Input {
    /// Path to the file to be deduplicated
    #[arg(short, long, group = "input")]
    pub path: Option<String>,

    /// Generate data with the given size (in MB) to deduplicate
    #[arg(short, long, group = "gen", value_name = "size")]
    pub generate: Option<usize>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum Algorithm {
    Ultra,
    Leap,
    Rabin
}
