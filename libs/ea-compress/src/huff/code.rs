//! The canonical Huffman code over byte values (spec §6).

use super::bits::BitReader;

/// Longest code a stream may declare (the length table must fill the code space by then).
pub(super) const MAX_CODE_LEN: u32 = 16;
/// Codes up to this many bits are decoded with a single table lookup.
const FAST_BITS: u32 = 10;

const LENS: usize = MAX_CODE_LEN as usize + 1;

pub(super) struct Code {
    /// Symbols in code order: shorter codes first, then by increasing code value.
    symbols: Vec<u8>,
    /// `first[len]`: the smallest code of `len` bits.
    first: [u32; LENS],
    /// `count[len]`: how many codes have `len` bits.
    count: [u32; LENS],
    /// `index[len]`: position in `symbols` of the first `len`-bit code.
    index: [u32; LENS],
    max_len: u32,
    /// Indexed by the next `FAST_BITS` bits: `symbol | len << 8`, or 0 for a longer code.
    fast: Vec<u16>,
}

impl Code {
    /// Code-length counts, then the symbols as skips over unused byte values.
    pub(super) fn read(bits: &mut BitReader<'_>) -> Result<Self, &'static str> {
        let mut code = Code {
            symbols: Vec::new(),
            first: [0; LENS],
            count: [0; LENS],
            index: [0; LENS],
            max_len: 0,
            fast: vec![0; 1 << FAST_BITS],
        };
        let total = code.read_lengths(bits)?;
        code.read_symbols(bits, total)?;
        code.build_fast_table();
        Ok(code)
    }

    /// Counts per length, until the codes exactly fill the code space. Returns the number of codes.
    fn read_lengths(&mut self, bits: &mut BitReader<'_>) -> Result<u32, &'static str> {
        let (mut next, mut total, mut len) = (0u32, 0u32, 0u32);
        while next != 1 << len {
            len += 1;
            if len > MAX_CODE_LEN {
                return Err("code lengths do not fill the code space");
            }
            let n = bits.number()?;
            next <<= 1;
            let l = len as usize;
            (self.first[l], self.count[l], self.index[l]) = (next, n, total);
            next += n;
            total += n;
            if next > 1 << len || total > 256 {
                return Err("too many codes");
            }
        }
        self.max_len = len;
        Ok(total)
    }

    /// Each symbol is the (skip + 1)-th byte value not used yet, counting upwards
    /// (wrapping) from the previous symbol; the first search starts at 0.
    fn read_symbols(&mut self, bits: &mut BitReader<'_>, total: u32) -> Result<(), &'static str> {
        let mut used = [false; 256];
        let mut value = u8::MAX;
        for i in 0..total {
            let mut skip = bits.number()? % (256 - i);
            loop {
                value = value.wrapping_add(1);
                if !used[usize::from(value)] {
                    if skip == 0 {
                        break;
                    }
                    skip -= 1;
                }
            }
            used[usize::from(value)] = true;
            self.symbols.push(value);
        }
        Ok(())
    }

    fn build_fast_table(&mut self) {
        for len in 1..=self.max_len.min(FAST_BITS) {
            let (l, shift) = (len as usize, FAST_BITS - len);
            for i in 0..self.count[l] {
                let c = self.first[l] + i;
                let entry = u16::from(self.symbols[(self.index[l] + i) as usize]) | ((len as u16) << 8);
                self.fast[(c << shift) as usize..((c + 1) << shift) as usize].fill(entry);
            }
        }
    }

    /// Decode one symbol.
    pub(super) fn decode(&self, bits: &mut BitReader<'_>) -> Result<u8, &'static str> {
        let entry = self.fast[bits.peek(FAST_BITS) as usize];
        if entry != 0 {
            bits.skip(u32::from(entry >> 8))?;
            return Ok(entry as u8);
        }
        let window = bits.peek(MAX_CODE_LEN);
        for len in FAST_BITS + 1..=self.max_len {
            let l = len as usize;
            let offset = (window >> (MAX_CODE_LEN - len)).wrapping_sub(self.first[l]);
            if offset < self.count[l] {
                bits.skip(len)?;
                return Ok(self.symbols[(self.index[l] + offset) as usize]);
            }
        }
        Err("invalid code")
    }
}
