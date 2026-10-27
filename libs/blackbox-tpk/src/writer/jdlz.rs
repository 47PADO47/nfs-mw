//! A JDLZ compressor, written from the stream description in `docs/formats/bchunk.md` section 3.
//!
//! Greedy LZ77 with a hash-chain matcher. The output is a valid stream for the decoder in `ea-compress`;
//! it is not byte-identical to what EA's tools produce, and does not need to be.

const MAGIC: &[u8; 4] = b"JDLZ";
const HEADER_LEN: usize = 16;

const MIN_MATCH: usize = 3;
/// Short form: distance 1..=16, length up to 0xFFF + 3.
const SHORT_MAX_DIST: usize = 16;
const SHORT_MAX_LEN: usize = 0xFFF + MIN_MATCH;
/// Long form: distance 17..=17 + 0x7FF, length up to 0x1F + 3.
const LONG_MIN_DIST: usize = 17;
const LONG_MAX_DIST: usize = LONG_MIN_DIST + 0x7FF;
const LONG_MAX_LEN: usize = 0x1F + MIN_MATCH;

const HASH_BITS: u32 = 15;
const MAX_CHAIN: usize = 32;

enum Op {
    Literal(u8),
    Copy { len: usize, dist: usize },
}

/// A flag register: the byte it lives in, and how many of its 8 bits are used.
struct FlagByte {
    at: usize,
    used: u32,
}

impl FlagByte {
    /// No byte yet: the first step starts one.
    fn new() -> Self {
        Self { at: 0, used: 8 }
    }

    fn exhausted(&self) -> bool {
        self.used == 8
    }

    fn start(&mut self, out: &mut Vec<u8>) {
        self.at = out.len();
        self.used = 0;
        out.push(0);
    }

    /// Set the next bit and move on.
    fn push(&mut self, out: &mut [u8], bit: bool) {
        out[self.at] |= u8::from(bit) << self.used;
        self.used += 1;
    }
}

fn hash3(b: &[u8]) -> usize {
    let v = u32::from(b[0]) | u32::from(b[1]) << 8 | u32::from(b[2]) << 16;
    (v.wrapping_mul(0x9E37_79B1) >> (32 - HASH_BITS)) as usize
}

/// Longest match for `input[pos..]` among earlier positions, as `(len, dist)`.
fn best_match(input: &[u8], pos: usize, head: &[u32], prev: &[u32]) -> Option<(usize, usize)> {
    if pos + MIN_MATCH > input.len() {
        return None;
    }
    let remaining = input.len() - pos;
    let mut best: Option<(usize, usize)> = None;
    let mut cand = head[hash3(&input[pos..])];
    let mut chain = 0;
    while cand != 0 && chain < MAX_CHAIN {
        let c = (cand - 1) as usize;
        let dist = pos - c;
        if dist > LONG_MAX_DIST {
            break;
        }
        let cap = if dist <= SHORT_MAX_DIST { SHORT_MAX_LEN } else { LONG_MAX_LEN }.min(remaining);
        let len = input[c..].iter().zip(&input[pos..]).take(cap).take_while(|(a, b)| a == b).count();
        let better = len >= MIN_MATCH && best.is_none_or(|(l, _)| len > l);
        if better {
            best = Some((len, dist));
        }
        cand = prev[c];
        chain += 1;
    }
    best
}

fn insert(input: &[u8], pos: usize, head: &mut [u32], prev: &mut [u32]) {
    if pos + MIN_MATCH > input.len() {
        return;
    }
    let h = hash3(&input[pos..]);
    prev[pos] = head[h];
    head[h] = pos as u32 + 1;
}

fn parse(input: &[u8]) -> Vec<Op> {
    let mut head = vec![0u32; 1 << HASH_BITS];
    let mut prev = vec![0u32; input.len()];
    let mut ops = Vec::new();
    let mut pos = 0;
    while pos < input.len() {
        let advance = match best_match(input, pos, &head, &prev) {
            Some((len, dist)) => {
                ops.push(Op::Copy { len, dist });
                len
            }
            None => {
                ops.push(Op::Literal(input[pos]));
                1
            }
        };
        for p in pos..pos + advance {
            insert(input, p, &mut head, &mut prev);
        }
        pos += advance;
    }
    ops
}

/// Write the ops the way the decoder reads them: before each step, a fresh `flags1` byte if its 8 bits are spent,
/// then a fresh `flags2` byte likewise (even when the step turns out to be a literal).
fn encode(ops: &[Op], out: &mut Vec<u8>) {
    let (mut f1, mut f2) = (FlagByte::new(), FlagByte::new());
    for op in ops {
        if f1.exhausted() {
            f1.start(out);
        }
        if f2.exhausted() {
            f2.start(out);
        }
        match *op {
            Op::Literal(b) => {
                out.push(b);
                f1.push(out, false);
            }
            Op::Copy { len, dist } => {
                let short = dist <= SHORT_MAX_DIST;
                let (b0, b1) = copy_bytes(len, dist, short);
                out.extend_from_slice(&[b0, b1]);
                f1.push(out, true);
                f2.push(out, short);
            }
        }
    }
}

fn copy_bytes(len: usize, dist: usize, short: bool) -> (u8, u8) {
    let l = len - MIN_MATCH;
    if short {
        return ((((l >> 8) << 4) | (dist - 1)) as u8, (l & 0xFF) as u8);
    }
    let d = dist - LONG_MIN_DIST;
    ((((d >> 8) << 5) | l) as u8, (d & 0xFF) as u8)
}

/// `input` as a complete JDLZ blob (16-byte header included).
pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() / 2 + HEADER_LEN);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&[0x02, 0x10, 0, 0]);
    out.extend_from_slice(&(input.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    encode(&parse(input), &mut out);
    let total = out.len() as u32;
    out[12..16].copy_from_slice(&total.to_le_bytes());
    out
}
