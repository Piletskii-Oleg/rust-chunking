use std::cmp::min;
use std::fmt::{self, Debug};
use std::io::{Result as IoResult, Seek, SeekFrom, Write};
use std::ptr;
use std::usize::MIN;

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
    pub fn new() -> Self {
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

/// Chunker
pub struct Chunker<W: Write + Seek> {
    dst: W,                // destination writer
    params: ChunkerParams, // chunker parameters
    pos: usize,
    chunk_len: usize,
    buf_clen: usize,
    win_idx: usize,
    roll_hash: u64,
    win: [u8; WINDOW_SIZE], // rolling hash circle window
    buf: Vec<u8>,           // chunker buffer, fixed size: WTR_BUF_LEN
}

impl<W: Write + Seek> Chunker<W> {
    pub fn new(params: ChunkerParams, dst: W) -> Self {
        let mut buf = vec![0u8; BUFFER_SIZE];
        buf.shrink_to_fit();

        Chunker {
            dst,
            params,
            pos: WINDOW_SLIDE_POS,
            chunk_len: WINDOW_SLIDE_POS,
            buf_clen: 0,
            win_idx: 0,
            roll_hash: 0,
            win: [0u8; WINDOW_SIZE],
            buf,
        }
    }

    pub fn into_inner(mut self) -> IoResult<W> {
        self.flush()?;
        Ok(self.dst)
    }
}

struct MyChunker {
    params: ChunkerParams, // chunker parameters
    pos: usize,
    chunk_len: usize,
    buf_clen: usize,
    win_idx: usize,
    roll_hash: u64,
    win: [u8; WINDOW_SIZE], // rolling hash circle window
    buf: Vec<u8>,
}

impl MyChunker {
    pub fn new(params: ChunkerParams) -> Self {
        let mut buf = vec![0u8; BUFFER_SIZE];
        buf.shrink_to_fit();

        MyChunker {
            params,
            pos: WINDOW_SLIDE_POS,
            chunk_len: WINDOW_SLIDE_POS,
            buf_clen: 0,
            win_idx: 0,
            roll_hash: 0,
            win: [0u8; WINDOW_SIZE],
            buf,
        }
    }

    fn generate_chunks(&mut self, data: &[u8]) -> Vec<Chunk> {
        if data.is_empty() {
            return vec![];
        }

        let mut chunks = vec![];

        let in_len = min(BUFFER_SIZE - self.buf_clen, data.len());
        self.buf[self.buf_clen..self.buf_clen + in_len].copy_from_slice(&data[..in_len]);
        self.buf_clen += in_len;

        while self.pos < self.buf_clen {
            let cur = self.buf[self.pos];
            let out = self.win[self.win_idx] as usize;
            let pushed_out = self.params.out_map[out];

            // Rabin rolling hash
            self.roll_hash = (self.roll_hash * PRIME) & MASK;
            self.roll_hash += u64::from(cur);
            self.roll_hash = self.roll_hash.wrapping_sub(pushed_out) & MASK;

            // window -> forward
            self.win[self.win_idx] = cur;
            self.win_idx = (self.win_idx + 1) & WINDOW_MASK;

            // move forward
            self.chunk_len += 1;
            self.pos += 1;

            // chunk can be written
            if self.chunk_len >= MIN_CHUNK_SIZE {
                let check_sum = self.roll_hash & self.params.ir[out];
                if (check_sum & CUT_MASK) == 0 || self.chunk_len >= MAX_CHUNK_SIZE {
                    chunks.push(Chunk::new(self.pos - self.chunk_len, self.chunk_len));

                    // not enough space in buffer, copy remaining to
                    // the head of buffer and reset buf position
                    if self.pos + MAX_CHUNK_SIZE >= BUFFER_SIZE {
                        let left_len = self.buf_clen - self.pos;
                        unsafe {
                            ptr::copy::<u8>(
                                self.buf[self.pos..].as_ptr(),
                                self.buf.as_mut_ptr(),
                                left_len,
                            );
                        }
                        self.buf_clen = left_len;
                        self.pos = 0;
                    }

                    // jump to next start sliding position
                    self.pos += WINDOW_SLIDE_POS;
                    self.chunk_len = WINDOW_SLIDE_POS;
                }
            }
        }

        chunks
    }
}

impl<W: Write + Seek> Write for Chunker<W> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let in_len = min(BUFFER_SIZE - self.buf_clen, buf.len());
        assert!(in_len > 0);
        self.buf[self.buf_clen..self.buf_clen + in_len].copy_from_slice(&buf[..in_len]);
        self.buf_clen += in_len;

        while self.pos < self.buf_clen {
            // get current byte and pushed out byte
            let ch = self.buf[self.pos];
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

            if self.chunk_len >= MIN_CHUNK_SIZE {
                let chksum = self.roll_hash ^ self.params.ir[out];

                // reached cut point, chunk can be produced now
                if (chksum & CUT_MASK) == 0 || self.chunk_len >= MAX_CHUNK_SIZE {
                    // write the chunk to destination writer,
                    // ensure it is consumed in whole
                    let p = self.pos - self.chunk_len;
                    let written = self.dst.write(&self.buf[p..self.pos])?;
                    assert_eq!(written, self.chunk_len);

                    // not enough space in buffer, copy remaining to
                    // the head of buffer and reset buf position
                    if self.pos + MAX_CHUNK_SIZE >= BUFFER_SIZE {
                        let left_len = self.buf_clen - self.pos;
                        unsafe {
                            ptr::copy::<u8>(
                                self.buf[self.pos..].as_ptr(),
                                self.buf.as_mut_ptr(),
                                left_len,
                            );
                        }
                        self.buf_clen = left_len;
                        self.pos = 0;
                    }

                    // jump to next start sliding position
                    self.pos += WINDOW_SLIDE_POS;
                    self.chunk_len = WINDOW_SLIDE_POS;
                }
            }
        }

        Ok(in_len)
    }

    fn flush(&mut self) -> IoResult<()> {
        // flush remaining data to destination
        let p = self.pos - self.chunk_len;
        if p < self.buf_clen {
            self.chunk_len = self.buf_clen - p;
            let _ = self.dst.write(&self.buf[p..(p + self.chunk_len)])?;
        }

        // reset chunker
        self.pos = WINDOW_SLIDE_POS;
        self.chunk_len = WINDOW_SLIDE_POS;
        self.buf_clen = 0;
        self.win_idx = 0;
        self.roll_hash = 0;
        self.win = [0u8; WINDOW_SIZE];

        self.dst.flush()
    }
}
