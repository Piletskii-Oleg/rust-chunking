use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::io::Write;

use crate::Chunk;
use serde::{Deserialize, Serialize};

// copied from https://github.com/zboxfs/zbox

// taken from pcompress implementation
// https://github.com/moinakg/pcompress
const PRIME: u64 = 153_191u64;
const MASK: u64 = 0x00ff_ffff_ffffu64;
const MIN_CHUNK_SIZE: usize = 16 * 1024; // minimal chunk size, 16k
const AVG_CHUNK_SIZE: usize = 32 * 1024; // average chunk size, 32k
const MAX_CHUNK_SIZE: usize = 64 * 1024; // maximum chunk size, 64k

// Irreducible polynomial for Rabin modulus, from pcompress
const FP_POLY: u64 = 0xbfe6_b8a5_bf37_8d83u64;

// since we will skip MIN_SIZE when sliding window, it only
// needs to target (AVG_SIZE - MIN_SIZE) cut length,
// note the (AVG_SIZE - MIN_SIZE) must be 2^n
const CUT_MASK: u64 = (AVG_CHUNK_SIZE - MIN_CHUNK_SIZE - 1) as u64;

// rolling hash window constants
const WINDOW_SIZE: usize = 16; // must be 2^n
const WINDOW_MASK: usize = WINDOW_SIZE - 1;
const WINDOW_SLIDE_OFFSET: usize = 64;
const WINDOW_SLIDE_POS: usize = MIN_CHUNK_SIZE - WINDOW_SLIDE_OFFSET;

// writer buffer length
const BUFFER_SIZE: usize = 8 * MAX_CHUNK_SIZE;

// QuickCDC constants
const KB: usize = 1024;
const FRONT_LENGTH: usize = 3;
const END_LENGTH: usize = 3;
const JUMP_LENGTH: usize = 8 * KB;

/// Pre-calculated chunker parameters
#[derive(Clone, Deserialize, Serialize)]
pub struct ChunkerParams {
    poly_pow: u64,     // poly power
    out_map: Vec<u64>, // pre-computed out byte map, length is 256
    ir: Vec<u64>,      // irreducible polynomial, length is 256
}

impl ChunkerParams {
    fn new() -> Self {
        let mut cp = ChunkerParams::default();

        // calculate poly power, it is actually PRIME ^ WIN_SIZE
        for _ in 0..WINDOW_SIZE {
            cp.poly_pow = (cp.poly_pow * PRIME) & MASK;
        }

        // pre-calculate out map table and irreducible polynomial
        // for each possible byte, copy from PCompress implementation
        for i in 0..256 {
            cp.out_map[i] = (i as u64 * cp.poly_pow) & MASK;

            let (mut term, mut pow, mut val) = (1u64, 1u64, 1u64);
            for _ in 0..WINDOW_SIZE {
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

impl Debug for ChunkerParams {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ChunkerParams()")
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

pub struct Chunker {
    params: ChunkerParams, // chunker parameters
    pos: usize,
    chunk_len: usize,
    win_idx: usize,
    roll_hash: u64,
    win: [u8; WINDOW_SIZE], // rolling hash circle window
    front: HashMap<[u8; 3], usize>,
    back: HashMap<[u8; 3], usize>,
}

impl Chunker {
    pub fn new() -> Self {

        Chunker {
            params: ChunkerParams::new(),
            pos: WINDOW_SLIDE_POS,
            chunk_len: WINDOW_SLIDE_POS,
            win_idx: 0,
            roll_hash: 0,
            win: [0u8; WINDOW_SIZE],
            front: HashMap::new(),
            back: HashMap::new(),
        }
    }

    fn check_chunk(&mut self, buf: &[u8]) -> Option<usize> {
        if self.pos + 3 > buf.len() {
            return None;
        }

        let front_range = self.pos..self.pos + 3;
        if let Some(front_length) = self.front.get(&buf[front_range]) {
            if self.pos + front_length > buf.len() {
                return None;
            }

            let end_range = self.pos + front_length - 3..self.pos + front_length;
            if let Some(end_length) = self.back.get(&buf[end_range]) {
                if *front_length == *end_length {
                    return Some(*front_length);
                }
            }
        }
        None
    }

    fn add_front_back(&mut self, buf: &[u8]) {
        let front_range = self.pos - self.chunk_len..self.pos - self.chunk_len + 3;
        let mut front_win = [0u8; 3];
        front_win.copy_from_slice(&buf[front_range]);
        self.front.insert(front_win, self.chunk_len);

        let end_range = self.pos - 3..self.pos;
        let mut end_win = [0u8; 3];
        end_win.copy_from_slice(&buf[end_range]);
        self.back.insert(end_win, self.chunk_len);
    }

    fn reset_hash(&mut self) {
        self.win_idx = 0;
        self.win = [0u8; WINDOW_SIZE];
        self.roll_hash = 0;
    }

    pub fn generate_chunks(&mut self, buf: &[u8]) -> Vec<Chunk> {
        if buf.is_empty() {
            return vec![];
        }

        let mut chunks = vec![];
        let mut counter = 0;

        while self.pos < buf.len() {
            if counter == 3 {
                self.reset_hash();
                counter = 0;
            }
            // get current byte and pushed out byte
            let ch = buf[self.pos];
            let out = self.win[self.win_idx] as usize;
            let pushed_out = self.params.out_map[out];

            // calculate Rabin rolling hash
            self.roll_hash = (self.roll_hash * PRIME) & MASK;
            self.roll_hash += u64::from(ch);
            self.roll_hash = self.roll_hash.wrapping_sub(pushed_out) & MASK;

            // forward circle window
            self.win[self.win_idx] = ch;
            self.win_idx = (self.win_idx + 1) & WINDOW_MASK;

            self.chunk_len += 1;
            self.pos += 1;

            // chunk can be written
            if self.chunk_len >= MIN_CHUNK_SIZE {
                let check_sum = self.roll_hash & self.params.ir[out];

                if (check_sum & CUT_MASK) == 0 || self.chunk_len >= MAX_CHUNK_SIZE {
                    chunks.push(Chunk::new(self.pos - self.chunk_len, self.chunk_len));
                    counter += 1;

                    // self.add_front_back(buf);
                    //
                    // while let Some(length) = self.check_chunk(buf) {
                    //     self.pos += length;
                    //     self.chunk_len = length;
                    //
                    //     chunks.push(Chunk::new(self.pos - self.chunk_len, self.chunk_len));
                    // }

                    // jump to next start sliding position
                    self.pos += WINDOW_SLIDE_POS;
                    self.chunk_len = WINDOW_SLIDE_POS;
                }
            }
        }

        let last_chunk = &chunks[chunks.len() - 1];
        if last_chunk.pos + last_chunk.len < buf.len() {
            let len = buf.len() - last_chunk.pos - last_chunk.len;
            chunks.push(Chunk::new(last_chunk.pos + last_chunk.len, len));
        }

        chunks
    }
}
