//! MSB-first bit reader and the variable-length number code (spec §4–§5).

/// Longest zero prefix accepted in a variable-length number (spec §5).
const MAX_NUMBER_ZEROS: u32 = 15;

/// Reads bits most-significant first. Errors are plain descriptions; the decoder
/// adds the stream positions.
pub(super) struct BitReader<'a> {
    input: &'a [u8],
    /// Next byte of `input` to load into `buf`. Runs past the end; those bytes read as 0.
    next: usize,
    /// Unread bits, left-aligned: the next bit is bit 63.
    buf: u64,
    /// Number of unread bits in `buf`.
    avail: u32,
}

impl<'a> BitReader<'a> {
    pub(super) fn new(input: &'a [u8]) -> Self {
        Self { input, next: 0, buf: 0, avail: 0 }
    }

    /// The next `n` bits (1..=32) without consuming them. Bits past the end read as 0.
    pub(super) fn peek(&mut self, n: u32) -> u32 {
        if self.avail < n {
            while self.avail <= 56 {
                let byte = self.input.get(self.next).copied().unwrap_or(0);
                self.buf |= u64::from(byte) << (56 - self.avail);
                self.next += 1;
                self.avail += 8;
            }
        }
        (self.buf >> (64 - n)) as u32
    }

    /// Consume `n` bits, at most as many as were just peeked.
    pub(super) fn skip(&mut self, n: u32) -> Result<(), &'static str> {
        self.buf <<= n;
        self.avail -= n;
        if self.bit_pos() > self.input.len() * 8 { Err("stream ended early") } else { Ok(()) }
    }

    pub(super) fn bits(&mut self, n: u32) -> Result<u32, &'static str> {
        let v = self.peek(n);
        self.skip(n)?;
        Ok(v)
    }

    /// Variable-length number: `z` zero bits, a one bit, then `z + 2` value bits
    /// `v`; the number is `v + 2^(z+2) - 4`.
    pub(super) fn number(&mut self) -> Result<u32, &'static str> {
        let zeros = self.peek(32).leading_zeros();
        if zeros > MAX_NUMBER_ZEROS {
            return Err("number has too many leading zero bits");
        }
        self.skip(zeros + 1)?;
        let width = zeros + 2;
        Ok(self.bits(width)? + (1 << width) - 4)
    }

    /// Bits consumed so far.
    pub(super) fn bit_pos(&self) -> usize {
        self.next * 8 - self.avail as usize
    }
}
