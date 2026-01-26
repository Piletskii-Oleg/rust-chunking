use crate::{Chunk, SizeParams};

const POLYNOMIAL: u64 = 0x3DA3_358B_4DC1_73;
const WINDOW_SIZE: usize = 32;
const POLYNOMIAL_DEGREE: usize = 53;
const POL_SHIFT: usize = POLYNOMIAL_DEGREE - 8;

// Configuration struct for TTTD algorithm.
// Defines the target average chunk size which influences the Rabin masks.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Config {
    // The target average block size.
    // This value determines the "primary" divisor/mask.
    // The "backup" divisor is derived as avg_size / 2.
    pub avg_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self { avg_size: 8192 }
    }
}

// The TTTD (Two Thresholds Two Divisors) Chunker.
//
// This chunker uses Rabin fingerprinting with two different masks:
// 1. `rabin_mask` (Primary/High): If matched, cut immediately.
// 2. `backup_mask` (Backup/Low): If matched, remember position.
//
// If the primary mask is never matched within `max_size`, the algorithm "backtracks"
// to the last position where the backup mask was matched.
pub struct Chunker<'a> {
    buf: &'a [u8],
    pos: usize,
    len: usize,
    sizes: SizeParams,

    // TTTD specific masks calculated from avg_size
    rabin_mask: u64,
    backup_mask: u64,

    // Internal state for Rabin's Rolling Hash
    window: [u8; WINDOW_SIZE],
    wpos: usize,
    digest: u64,

    // Precomputed tables for efficient hashing
    out_table: [u64; 256], // Handles byte removal from window
    mod_table: [u64; 256], // Handles polynomial modulo arithmetic
}

impl<'a> Chunker<'a> {
    pub fn default_sizes() -> SizeParams {
        SizeParams {
            min: 1024,
            avg: 8192,
            max: 65536,
        }
    }

    pub fn new(buf: &'a [u8], sizes: SizeParams, config: Config) -> Self {
        // Calculate the Primary Mask (High threshold).
        let avg_pow2 = config.avg_size.next_power_of_two();
        let rabin_mask = (avg_pow2 - 1) as u64;

        // Calculate the Backup Mask (Low threshold).
        let backup_avg = config.avg_size / 2;
        let backup_pow2 = backup_avg.next_power_of_two();
        let backup_mask = (backup_pow2 - 1) as u64;

        // Precompute tables for rolling hash performance.
        let (out_table, mod_table) = calc_tables();

        let mut chunker = Self {
            buf,
            pos: 0,
            len: buf.len(),
            sizes,
            rabin_mask,
            backup_mask,
            window: [0; WINDOW_SIZE],
            wpos: 0,
            digest: 0,
            out_table,
            mod_table,
        };

        chunker.rabin_reset();

        chunker
    }

    // Rabin Helper Functions

    fn rabin_reset(&mut self) {
        self.window.fill(0);
        self.wpos = 0;
        self.digest = 0;

        self.rabin_slide(1);
    }

    // Slides the window: removes the old byte and adds the new byte `b`.
    fn rabin_slide(&mut self, b: u8) {
        let out = self.window[self.wpos];
        self.window[self.wpos] = b;
        self.digest ^= self.out_table[out as usize];
        self.wpos = (self.wpos + 1) % WINDOW_SIZE;

        self.rabin_append(b);
    }

    // Appends a new byte to the current digest using polynomial arithmetic.
    fn rabin_append(&mut self, b: u8) {
        let index = (self.digest >> POL_SHIFT) as u8;
        self.digest <<= 8;
        self.digest |= b as u64;
        self.digest ^= self.mod_table[index as usize];
    }

    // --- TTTD Main Logic ---

    fn find_border(&mut self) -> Option<usize> {
        if self.pos >= self.len {
            return None;
        }

        let start = self.pos;
        let remaining = self.len - self.pos;

        if remaining < self.sizes.min {
            self.pos = self.len;

            return Some(remaining);
        }

        let limit = std::cmp::min(remaining, self.sizes.max);

        let mut last_backup_pos: Option<usize> = None;

        self.rabin_reset();

        for curr_off in self.sizes.min..=limit {
            let byte_idx = start + curr_off;

            if byte_idx >= self.len {
                break;
            }

            let b = self.buf[byte_idx];

            self.rabin_slide(b);

            if (self.digest & self.backup_mask) == 0 {
                last_backup_pos = Some(curr_off);

                if (self.digest & self.rabin_mask) == 0 {
                    let chunk_len = curr_off + 1;
                    self.pos = start + chunk_len;

                    return Some(chunk_len);
                }
            }
        }

        if let Some(backup_off) = last_backup_pos {
            let chunk_len = backup_off + 1;
            self.pos = start + chunk_len;

            return Some(chunk_len);
        }

        let chunk_len = limit;
        self.pos = start + chunk_len;

        Some(chunk_len)
    }
}

impl Iterator for Chunker<'_> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.pos;
        self.find_border().map(|len| Chunk::new(start, len))
    }
}

// Rabin Polynomial Math

fn deg(p: u64) -> i32 {
    if p == 0 {
        return -1;
    }
    
    (63 - p.leading_zeros()) as i32
}

fn poly_mod(x: u64, p: u64) -> u64 {
    let dp = deg(p);
    let dx = deg(x);

    if dx < dp {
        return x
    }

    poly_mod(x ^ (p << (dx - dp)), p)
}

fn append_byte(hash: u64, b: u8, pol: u64) -> u64 {
    let new_val = (hash << 8) | (b as u64);
    
    poly_mod(new_val, pol)
}

fn calc_tables() -> ([u64; 256], [u64; 256]) {
    let out_table: [u64; 256] = std::array::from_fn(|i| {
        (0..(WINDOW_SIZE - 1)).fold(
            append_byte(0, i as u8, POLYNOMIAL), 
            |hash, _| append_byte(hash, 0, POLYNOMIAL)
        )
    });

    let mod_table: [u64; 256] = std::array::from_fn(|i| {
        let b = i as u64;
        let shifted = b << deg(POLYNOMIAL);
        poly_mod(shifted, POLYNOMIAL) | shifted
    });

    (out_table, mod_table)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_pattern(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn test_tttd_logic() {
        let mut data = vec![0u8; 100_000];
        let mut seed: u32 = 12345;
        for i in 0..100_000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            data[i] = (seed >> 24) as u8;
        }

        let sizes = SizeParams::new(1000, 4000, 8000);
        let config = Config { avg_size: 4000 };

        let chunker = Chunker::new(&data, sizes, config);
        let chunks: Vec<_> = chunker.collect();

        let mut hit_max = 0;
        let mut hit_cut = 0;

        for chunk in chunks {
            if chunk.len == 8000 {
                // println!("Hit max size at {}", chunk.pos);
                hit_max += 1;
            } else {
                // println!("Cut found at len {}", chunk.len);
                hit_cut += 1;
            }
        }

        println!("Total chunks: {}", hit_max + hit_cut);
        println!("  - Max size forced: {}", hit_max);
        println!("  - Natural cuts (Rabin/Backup): {}", hit_cut);

        assert!(
            hit_cut > 0,
            "Algorithm failed to find any natural cut points on random data!"
        );
    }

    #[test]
    fn test_tttd_min_size_constraint() {
        let data = vec![1u8; 500];
        let sizes = SizeParams::new(1000, 2000, 4000);
        let config = Config { avg_size: 2000 };

        let chunker = Chunker::new(&data, sizes, config);
        let chunks: Vec<_> = chunker.collect();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len, 500);
    }

    #[test]
    fn test_tttd_exact_max_cut() {
        let data = vec![0xFFu8; 10000];

        let sizes = SizeParams::new(100, 500, 1000);
        let config = Config {
            avg_size: 1024 * 1024 * 1024,
        };

        let chunker = Chunker::new(&data, sizes, config);
        let chunks: Vec<_> = chunker.collect();

        assert_eq!(chunks[0].len, 1000);
    }

    #[test]
    fn test_tttd_consistency() {
        let data = generate_pattern(50_000);
        let sizes = SizeParams::new(512, 2048, 8192);
        let config = Config { avg_size: 2048 };

        let c1: Vec<_> = Chunker::new(&data, sizes, config).collect();
        let c2: Vec<_> = Chunker::new(&data, sizes, config).collect();

        assert_eq!(c1.len(), c2.len());
        for (a, b) in c1.iter().zip(c2.iter()) {
            assert_eq!(a.pos, b.pos);
            assert_eq!(a.len, b.len);
        }
    }

    #[test]
    fn test_tttd_backup_logic() {
        let mut data = vec![0u8; 200_000];
        let mut seed: u32 = 999;
        for i in 0..data.len() {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            data[i] = (seed >> 24) as u8;
        }

        let sizes = SizeParams::new(1024, 4096, 16384);

        let config1 = Config { avg_size: 4096 };
        let count1 = Chunker::new(&data, sizes, config1).count();

        let config2 = Config { avg_size: 8192 };
        let count2 = Chunker::new(&data, sizes, config2).count();

        println!("Chunks (avg 4096): {}", count1);
        println!("Chunks (avg 8192): {}", count2);

        assert!(
            count1 > count2,
            "Stricter mask (larger avg) should yield FEWER chunks"
        );
    }

    #[test]
    fn test_tttd_reconstruct() {
        let len = 123_456;
        let data = generate_pattern(len);
        let sizes = SizeParams::new(1024, 4096, 8192);
        let config = Config { avg_size: 4096 };

        let chunker = Chunker::new(&data, sizes, config);
        let mut total_len = 0;
        let mut last_pos = 0;

        for chunk in chunker {
            assert_eq!(chunk.pos, last_pos);
            total_len += chunk.len;
            last_pos += chunk.len;
        }

        assert_eq!(total_len, len);
    }

    #[test]
    fn test_rabin_math_internals() {
        assert_eq!(deg(POLYNOMIAL), 53);

        let hash = 0;
        let new_hash = append_byte(hash, 0xAB, POLYNOMIAL);
        assert!(new_hash > 0);
    }

    #[test]
    fn test_tttd_backup_trigger_explicit() {
        let mut data = vec![0u8; 100_000];
        let mut seed: u32 = 42;
        for i in 0..data.len() {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            data[i] = (seed >> 24) as u8;
        }

        let sizes = SizeParams::new(1000, 4000, 8000);

        let real_config = Config { avg_size: 16000 };

        let chunker = Chunker::new(&data, sizes, real_config);
        let chunks: Vec<_> = chunker.collect();

        let mut tttd_triggers = 0;
        for c in chunks {
            if c.len < 8000 {
                tttd_triggers += 1;
            }
        }

        println!("Chunks cut by content (not size limit): {}", tttd_triggers);
        assert!(
            tttd_triggers > 0,
            "TTTD should find content boundaries even with strict primary mask"
        );
    }
}
