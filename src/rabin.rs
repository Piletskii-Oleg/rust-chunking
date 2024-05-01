use crate::Chunk;

// implementation taken from zbox
// https://github.com/zboxfs/zbox

// taken from pcompress implementation
// https://github.com/moinakg/pcompress
const PRIME: u64 = 153_191u64;
const MASK: u64 = 0x00ff_ffff_ffffu64;
const MIN_SIZE: usize = 16 * 1024; // minimal chunk size, 16k
const AVG_SIZE: usize = 32 * 1024; // average chunk size, 32k
const MAX_SIZE: usize = 64 * 1024; // maximum chunk size, 64k

// Irreducible polynomial for Rabin modulus, from pcompress
const FP_POLY: u64 = 0xbfe6_b8a5_bf37_8d83u64;

// since we will skip MIN_SIZE when sliding window, it only
// needs to target (AVG_SIZE - MIN_SIZE) cut length,
// note the (AVG_SIZE - MIN_SIZE) must be 2^n
const CUT_MASK: u64 = (AVG_SIZE - MIN_SIZE - 1) as u64;

// rolling hash window constants
const WIN_SIZE: usize = 16; // must be 2^n
const WIN_MASK: usize = WIN_SIZE - 1;
const WIN_SLIDE_OFFSET: usize = 64;
const WIN_SLIDE_POS: usize = MIN_SIZE - WIN_SLIDE_OFFSET;

pub struct Chunker<'a> {
    buf: &'a [u8],
    params: ChunkerParams, // chunker parameters
    pos: usize,
    len: usize,
}

/// Pre-calculated chunker parameters
#[derive(Clone)]
pub struct ChunkerParams {
    poly_pow: u64,     // poly power
    out_map: Vec<u64>, // pre-computed out byte map, length is 256
    ir: Vec<u64>,      // irreducible polynomial, length is 256
}

impl<'a> Chunker<'a> {
    pub fn new(buf: &'a [u8]) -> Chunker {
        Chunker {
            buf,
            pos: 0,
            params: ChunkerParams::new(),
            len: buf.len(),
        }
    }

    pub fn with_params(buf: &'a [u8], params: ChunkerParams) -> Self {
        Self {
            buf,
            params,
            pos: 0,
            len: buf.len(),
        }
    }

    fn find_border(&mut self) -> Option<usize> {
        if self.len == self.pos {
            return None;
        }

        if self.len - self.pos < MIN_SIZE {
            let pos = self.pos;
            self.pos = self.len;
            return Some(self.len - pos);
        }

        self.pos += WIN_SLIDE_POS;
        let mut chunk_len = WIN_SLIDE_POS;

        let mut win = [0u8; WIN_SIZE];
        let mut win_idx = 0;
        let mut roll_hash = 0;

        while self.pos < self.len {
            let ch = self.buf[self.pos];
            let out = win[win_idx] as usize;
            let pushed_out = self.params.out_map[out];

            // calculate Rabin rolling hash
            roll_hash = (roll_hash * PRIME) & MASK;
            roll_hash += u64::from(ch);
            roll_hash = roll_hash.wrapping_sub(pushed_out) & MASK;

            // forward circle window
            win[win_idx] = ch;
            win_idx = (win_idx + 1) & WIN_MASK;

            chunk_len += 1;
            self.pos += 1;

            if chunk_len >= MIN_SIZE {
                let checksum = roll_hash ^ self.params.ir[out];

                if (checksum & CUT_MASK) == 0 || chunk_len >= MAX_SIZE {
                    return Some(chunk_len);
                }
            }
        }

        Some(chunk_len)
    }

    pub fn give_params(self) -> ChunkerParams {
        self.params
    }
}

impl<'a> Iterator for Chunker<'a> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.pos;

        self.find_border().map(|length| Chunk::new(start, length))
    }
}

impl ChunkerParams {
    pub fn new() -> Self {
        let mut cp = ChunkerParams::default();

        // calculate poly power, it is actually PRIME ^ WIN_SIZE
        for _ in 0..WIN_SIZE {
            cp.poly_pow = (cp.poly_pow * PRIME) & MASK;
        }

        // pre-calculate out map table and irreducible polynomial
        // for each possible byte, copy from PCompress implementation
        for i in 0..256 {
            cp.out_map[i] = (i as u64 * cp.poly_pow) & MASK;

            let (mut term, mut pow, mut val) = (1u64, 1u64, 1u64);
            for _ in 0..WIN_SIZE {
                if (term & FP_POLY) != 0 {
                    val += (pow * i as u64) & MASK;
                }
                pow = (pow * PRIME) & MASK;
                term *= 2;
            }
            cp.ir[i] = val;
        }

        cp
    }
}

impl Default for ChunkerParams {
    fn default() -> Self {
        let mut ret = ChunkerParams {
            poly_pow: 1,
            out_map: vec![0u64; 256],
            ir: vec![0u64; 256],
        };
        ret.out_map.shrink_to_fit();
        ret.ir.shrink_to_fit();
        ret
    }
}
