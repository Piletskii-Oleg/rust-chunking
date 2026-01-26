use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use cdc_chunkers::{tttd, SizeParams};

fn generate_random_data(size: usize) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(42);
    let mut data = vec![0u8; size];
    rng.fill(&mut data[..]);
    data
}

fn generate_pattern_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 251) as u8).collect()
}

fn bench_tttd(c: &mut Criterion) {
    let size_10mb = 10 * 1024 * 1024;
    
    let random_data = generate_random_data(size_10mb);
    let zero_data = vec![0u8; size_10mb];
    let pattern_data = generate_pattern_data(size_10mb);
    
    let sizes = SizeParams::tttd_default(); // min=1024, avg=8192, max=65536
    let config = tttd::Config::default();   // avg_size=8192

    let mut group = c.benchmark_group("TTTD Chunking");
    
    group.throughput(Throughput::Bytes(size_10mb as u64));

    group.bench_function("Random Data (10MB)", |b| {
        b.iter(|| {
            let chunker = tttd::Chunker::new(black_box(&random_data), sizes, config);
            let count = chunker.count();
            black_box(count);
        })
    });

    group.bench_function("Zero Data (10MB)", |b| {
        b.iter(|| {
            let chunker = tttd::Chunker::new(black_box(&zero_data), sizes, config);
            let count = chunker.count();
            black_box(count);
        })
    });

    group.bench_function("Pattern Data (10MB)", |b| {
        b.iter(|| {
            let chunker = tttd::Chunker::new(black_box(&pattern_data), sizes, config);
            let count = chunker.count();
            black_box(count);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_tttd);
criterion_main!(benches);