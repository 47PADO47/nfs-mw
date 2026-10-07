//! HUFF: EA's canonical-Huffman + run-length codec (EA compression-library
//! stream type `0x30FB`), used for most compressed TPK textures.
//!
//! Implemented from our own spec, `docs/formats/huff.md`. No third-party code
//! was copied; the sources read to write that spec are listed there.

use crate::{Error, HEADER_LEN, HUFF_MAGIC, Header, Result};

const KIND: &str = "HUFF";

/// Longest code a stream may declare (the length table must fill the code space by then).
const MAX_CODE_LEN: u32 = 16;
/// Codes up to this many bits are decoded with a single table lookup.
const FAST_BITS: u32 = 10;
/// Longest zero prefix accepted in a variable-length number (spec §5).
const MAX_NUMBER_ZEROS: u32 = 15;

fn bad_header(detail: String) -> Error {
    Error::BadHeader { kind: KIND, detail }
}

/// Decompress a HUFF blob (16-byte `HUFF` wrapper header followed by the EA stream).
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let h = Header::parse(data, KIND)?;
    if h.magic != HUFF_MAGIC || h.version != 0x01 || h.flags != 0x10 {
        return Err(bad_header(format!("magic/version/flags {:08X}/{:02X}/{:02X}", h.magic, h.version, h.flags)));
    }
    // Unlike JDLZ's, this size field does not include the 16-byte header.
    let end = HEADER_LEN.saturating_add(h.compressed_size as usize).min(data.len());
    Decoder::new(&data[HEADER_LEN..end]).run(h.decompressed_size as usize)
}

/// Post-filter selected by the stream type (spec §8).
#[derive(Debug, Clone, Copy)]
enum Filter {
    /// `30FB`: bytes are stored as is.
    Plain,
    /// `32FB`: bytes are stored as differences; output is their running sum.
    Delta,
    /// `34FB`: differences of differences; output is the running sum of the running sum.
    DeltaDelta,
}

impl Filter {
    fn apply(self, out: &mut [u8]) {
        match self {
            Self::Plain => {}
            Self::Delta => {
                let mut sum = 0u8;
                for b in out {
                    sum = sum.wrapping_add(*b);
                    *b = sum;
                }
            }
            Self::DeltaDelta => {
                let (mut delta, mut sum) = (0u8, 0u8);
                for b in out {
                    delta = delta.wrapping_add(*b);
                    sum = sum.wrapping_add(delta);
                    *b = sum;
                }
            }
        }
    }
}

/// A canonical Huffman code over byte values (spec §6).
struct Code {
    /// Symbols in code order: shorter codes first, then by increasing code value.
    symbols: Vec<u8>,
    /// `first[len]`: the smallest code of `len` bits.
    first: [u32; MAX_CODE_LEN as usize + 1],
    /// `count[len]`: how many codes have `len` bits.
    count: [u32; MAX_CODE_LEN as usize + 1],
    /// `index[len]`: position in `symbols` of the first `len`-bit code.
    index: [u32; MAX_CODE_LEN as usize + 1],
    max_len: u32,
    /// Indexed by the next `FAST_BITS` bits: `symbol | len << 8`, or 0 for a longer code.
    fast: Vec<u16>,
}

/// MSB-first bit reader over the EA stream, plus the output so errors can report both positions.
struct Decoder<'a> {
    input: &'a [u8],
    /// Next byte of `input` to load into `buf`. Runs past the end; those bytes read as 0.
    next: usize,
    /// Unread bits, left-aligned: the next bit is bit 63.
    buf: u64,
    /// Number of unread bits in `buf`.
    avail: u32,
    out: Vec<u8>,
}

impl<'a> Decoder<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, next: 0, buf: 0, avail: 0, out: Vec::new() }
    }

    fn run(mut self, expected: usize) -> Result<Vec<u8>> {
        let (filter, size) = self.stream_header()?;
        if size != expected {
            return Err(bad_header(format!("stream size {size} does not match wrapper size {expected}")));
        }
        let clue = self.bits(8)? as u8;
        let code = self.code()?;
        self.out.reserve_exact(size);

        loop {
            let byte = self.symbol(&code)?;
            if byte != clue {
                self.emit(byte, size)?;
                continue;
            }
            // The clue symbol introduces a run, an escaped byte or the end of the stream.
            let run = self.number()? as usize;
            if run > 0 {
                let &last = self.out.last().ok_or_else(|| self.corrupt("run before the first byte"))?;
                if run > size - self.out.len() {
                    return Err(self.corrupt("run past the decompressed size"));
                }
                self.out.resize(self.out.len() + run, last);
            } else if self.bits(1)? == 1 {
                break;
            } else {
                let byte = self.bits(8)? as u8;
                self.emit(byte, size)?;
            }
        }

        if self.out.len() != size {
            return Err(self.corrupt("end of stream before the decompressed size"));
        }
        filter.apply(&mut self.out);
        Ok(self.out)
    }

    /// Type id and sizes (spec §3). Returns the filter and this stream's decompressed size.
    fn stream_header(&mut self) -> Result<(Filter, usize)> {
        let id = self.bits(16)?;
        let filter = match id & !0x8100 {
            0x30FB => Filter::Plain,
            0x32FB => Filter::Delta,
            0x34FB => Filter::DeltaDelta,
            _ => {
                return Err(bad_header(format!("stream type {id:04X} is not a Huffman type (30FB-35FB, B0FB-B5FB)")));
            }
        };
        let width = if id & 0x8000 != 0 { 32 } else { 24 };
        if id & 0x0100 != 0 {
            self.bits(width)?; // total size of a multi-stream ("composite") object
        }
        Ok((filter, self.bits(width)? as usize))
    }

    /// Code-length counts, then the symbols as skips over unused byte values (spec §6).
    fn code(&mut self) -> Result<Code> {
        let mut code = Code {
            symbols: Vec::new(),
            first: [0; MAX_CODE_LEN as usize + 1],
            count: [0; MAX_CODE_LEN as usize + 1],
            index: [0; MAX_CODE_LEN as usize + 1],
            max_len: 0,
            fast: vec![0; 1 << FAST_BITS],
        };

        // Counts per length, until the codes exactly fill the code space.
        let (mut next, mut total, mut len) = (0u32, 0u32, 0u32);
        while next != 1 << len {
            len += 1;
            if len > MAX_CODE_LEN {
                return Err(self.corrupt("code lengths do not fill the code space"));
            }
            let n = self.number()?;
            next <<= 1;
            let l = len as usize;
            (code.first[l], code.count[l], code.index[l]) = (next, n, total);
            next += n;
            total += n;
            if next > 1 << len || total > 256 {
                return Err(self.corrupt("too many codes"));
            }
        }
        code.max_len = len;

        // Symbols: each is the (skip + 1)-th byte value not used yet, counting
        // upwards (wrapping) from the previous symbol; the first search starts at 0.
        let mut used = [false; 256];
        let mut value = u8::MAX;
        for i in 0..total {
            let mut skip = self.number()? % (256 - i);
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
            code.symbols.push(value);
        }

        for len in 1..=code.max_len.min(FAST_BITS) {
            let (l, shift) = (len as usize, FAST_BITS - len);
            for i in 0..code.count[l] {
                let c = code.first[l] + i;
                let entry = u16::from(code.symbols[(code.index[l] + i) as usize]) | ((len as u16) << 8);
                code.fast[(c << shift) as usize..((c + 1) << shift) as usize].fill(entry);
            }
        }
        Ok(code)
    }

    fn symbol(&mut self, code: &Code) -> Result<u8> {
        let entry = code.fast[self.peek(FAST_BITS) as usize];
        if entry != 0 {
            self.skip(u32::from(entry >> 8))?;
            return Ok(entry as u8);
        }
        let window = self.peek(MAX_CODE_LEN);
        for len in FAST_BITS + 1..=code.max_len {
            let l = len as usize;
            let offset = (window >> (MAX_CODE_LEN - len)).wrapping_sub(code.first[l]);
            if offset < code.count[l] {
                self.skip(len)?;
                return Ok(code.symbols[(code.index[l] + offset) as usize]);
            }
        }
        Err(self.corrupt("invalid code"))
    }

    /// Variable-length number (spec §5): `z` zero bits, a one bit, then `z + 2`
    /// value bits `v`; the number is `v + 2^(z+2) - 4`.
    fn number(&mut self) -> Result<u32> {
        let zeros = self.peek(32).leading_zeros();
        if zeros > MAX_NUMBER_ZEROS {
            return Err(self.corrupt("number has too many leading zero bits"));
        }
        self.skip(zeros + 1)?;
        let width = zeros + 2;
        Ok(self.bits(width)? + (1 << width) - 4)
    }

    fn emit(&mut self, byte: u8, size: usize) -> Result<()> {
        if self.out.len() == size {
            return Err(self.corrupt("output past the decompressed size"));
        }
        self.out.push(byte);
        Ok(())
    }

    /// The next `n` bits (1..=32) without consuming them. Bits past the end read as 0.
    fn peek(&mut self, n: u32) -> u32 {
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
    fn skip(&mut self, n: u32) -> Result<()> {
        self.buf <<= n;
        self.avail -= n;
        if self.bit_pos() > self.input.len() * 8 {
            return Err(self.corrupt("stream ended early"));
        }
        Ok(())
    }

    fn bits(&mut self, n: u32) -> Result<u32> {
        let v = self.peek(n);
        self.skip(n)?;
        Ok(v)
    }

    /// Bits consumed so far.
    fn bit_pos(&self) -> usize {
        self.next * 8 - self.avail as usize
    }

    fn corrupt(&self, detail: &'static str) -> Error {
        Error::Corrupt { kind: KIND, input: HEADER_LEN + self.bit_pos() / 8, output: self.out.len(), detail }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MSB-first bit writer for hand-built streams.
    #[derive(Default)]
    struct BitWriter {
        bytes: Vec<u8>,
        len: usize,
    }

    impl BitWriter {
        fn put(&mut self, value: u32, width: u32) -> &mut Self {
            for i in (0..width).rev() {
                if self.len.is_multiple_of(8) {
                    self.bytes.push(0);
                }
                if (value >> i) & 1 != 0 {
                    *self.bytes.last_mut().unwrap() |= 0x80 >> (self.len % 8);
                }
                self.len += 1;
            }
            self
        }

        /// Variable-length number: `width - 2` zeros, a one, then `width` value bits.
        fn num(&mut self, v: u32) -> &mut Self {
            let mut width = 2;
            while v + 4 >= 1 << (width + 1) {
                width += 1;
            }
            self.put(1, width - 1).put(v + 4 - (1 << width), width)
        }

        // The test code: 'a' = 0, 'b' = 10, 'c' = 110, clue 'z' = 111.
        fn text(&mut self, text: &[u8]) -> &mut Self {
            for &c in text {
                match c {
                    b'a' => self.put(0, 1),
                    b'b' => self.put(0b10, 2),
                    b'c' => self.put(0b110, 3),
                    _ => unreachable!(),
                };
            }
            self
        }

        fn run(&mut self, n: u32) -> &mut Self {
            self.put(0b111, 3).num(n)
        }

        fn escaped(&mut self, byte: u8) -> &mut Self {
            self.put(0b111, 3).num(0).put(0, 1).put(byte.into(), 8)
        }

        fn end(&mut self) -> &mut Self {
            self.put(0b111, 3).num(0).put(0b10, 2)
        }

        /// Clue 'z' and the test code table: one 1-bit, one 2-bit and two 3-bit codes.
        fn abc_table(&mut self) -> &mut Self {
            self.put(b'z'.into(), 8);
            for n in [1, 1, 2] {
                self.num(n);
            }
            // 'a' skips 0x00..=0x60; 'b', 'c' are next; 'z' skips 0x64..=0x79.
            for skip in [0x61, 0, 0, 0x16] {
                self.num(skip);
            }
            self
        }
    }

    fn blob(stream: &[u8], out_size: u32) -> Vec<u8> {
        let mut v = b"HUFF".to_vec();
        v.extend_from_slice(&[0x01, 0x10, 0, 0]);
        v.extend_from_slice(&out_size.to_le_bytes());
        v.extend_from_slice(&(stream.len() as u32).to_le_bytes());
        v.extend_from_slice(stream);
        v
    }

    /// A plain `30FB` blob using the test code.
    fn abc_blob(size: u32, body: impl FnOnce(&mut BitWriter)) -> Vec<u8> {
        let mut w = BitWriter::default();
        w.put(0x30FB, 16).put(size, 24).abc_table();
        body(&mut w);
        blob(&w.bytes, size)
    }

    #[test]
    fn literals() {
        let data = abc_blob(5, |w| {
            w.text(b"abcab").end();
        });
        assert_eq!(decompress(&data).unwrap(), b"abcab");
    }

    #[test]
    fn run_repeats_previous_byte() {
        let data = abc_blob(7, |w| {
            w.text(b"a").run(5).text(b"b").end();
        });
        assert_eq!(decompress(&data).unwrap(), b"aaaaaab");
    }

    #[test]
    fn escaped_bytes() {
        // The clue value itself and a byte with no code.
        let data = abc_blob(3, |w| {
            w.escaped(b'z').escaped(0x00).text(b"c").end();
        });
        assert_eq!(decompress(&data).unwrap(), b"z\0c");
    }

    #[test]
    fn number_widths() {
        // Run lengths at each edge of the 3-, 5-, 7- and 9-bit forms, plus a long one.
        let runs = [1, 3, 4, 11, 12, 27, 28, 30_000];
        let mut expected = Vec::new();
        for (i, &r) in runs.iter().enumerate() {
            let c = b"ab"[i % 2];
            expected.extend(std::iter::repeat_n(c, r as usize + 1));
        }
        let data = abc_blob(expected.len() as u32, |w| {
            for (i, &r) in runs.iter().enumerate() {
                w.text(&[b"ab"[i % 2]]).run(r);
            }
            w.end();
        });
        assert_eq!(decompress(&data).unwrap(), expected);
    }

    #[test]
    fn delta_filters() {
        let stream = |id: u32, bytes: &[u8]| {
            let mut w = BitWriter::default();
            w.put(id, 16).put(bytes.len() as u32, 24).abc_table();
            for &b in bytes {
                w.escaped(b);
            }
            w.end();
            blob(&w.bytes, bytes.len() as u32)
        };
        assert_eq!(decompress(&stream(0x32FB, &[1, 1, 1, 0xFF])).unwrap(), [1, 2, 3, 2]);
        assert_eq!(decompress(&stream(0x34FB, &[1, 1, 1])).unwrap(), [1, 3, 6]);
    }

    #[test]
    fn wide_and_composite_headers() {
        for (id, width) in [(0x31FB, 24), (0xB0FB, 32), (0xB1FB, 32)] {
            let mut w = BitWriter::default();
            w.put(id, 16);
            if id & 0x100 != 0 {
                w.put(1000, width); // composite total, ignored
            }
            w.put(2, width).abc_table().text(b"ba").end();
            assert_eq!(decompress(&blob(&w.bytes, 2)).unwrap(), b"ba", "type {id:04X}");
        }
    }

    #[test]
    fn long_codes() {
        // Symbols 0..=12: one code of each length 1..=11, two of length 12.
        // Symbol n < 11 has code 2^(n+1) - 2; 11 is 0xFFE; the clue 12 is 0xFFF.
        let mut w = BitWriter::default();
        w.put(0x30FB, 16).put(4, 24).put(12, 8);
        for len in 1..=12 {
            w.num(if len == 12 { 2 } else { 1 });
        }
        for _ in 0..=12 {
            w.num(0);
        }
        w.put(0x7FE, 11).put(0xFFE, 12).put(0, 1).put(0x3FE, 10);
        w.put(0xFFF, 12).num(0).put(0b10, 2);
        assert_eq!(decompress(&blob(&w.bytes, 4)).unwrap(), [10, 11, 0, 9]);
    }

    fn assert_corrupt(data: &[u8]) {
        let err = decompress(data).unwrap_err();
        assert!(matches!(err, Error::Corrupt { .. }), "{err}");
    }

    #[test]
    fn run_before_first_byte_is_error() {
        assert_corrupt(&abc_blob(3, |w| {
            w.run(3).end();
        }));
    }

    #[test]
    fn wrong_output_length_is_error() {
        assert_corrupt(&abc_blob(2, |w| {
            w.text(b"abc").end();
        }));
        assert_corrupt(&abc_blob(4, |w| {
            w.text(b"abc").end();
        }));
        assert_corrupt(&abc_blob(4, |w| {
            w.text(b"a").run(4).end();
        }));
    }

    #[test]
    fn truncated_stream_is_error() {
        let mut data = abc_blob(5, |w| {
            w.text(b"abcab").end();
        });
        // The last byte holds the end-of-stream bit.
        let size = data.len() as u32 - 16;
        data[12..16].copy_from_slice(&(size - 1).to_le_bytes());
        assert_corrupt(&data);
        data[12..16].copy_from_slice(&size.to_le_bytes());
        data.pop();
        assert_corrupt(&data);
        // Header only.
        assert_corrupt(&blob(&[], 1));
    }

    #[test]
    fn bad_code_tables_are_errors() {
        let table = |counts: &[u32]| {
            let mut w = BitWriter::default();
            w.put(0x30FB, 16).put(1, 24).put(0, 8);
            for &n in counts {
                w.num(n);
            }
            w.put(0, 32);
            blob(&w.bytes, 1)
        };
        assert_corrupt(&table(&[3]));
        assert_corrupt(&table(&[1; 17]));
        // A number with more than 15 leading zeros.
        assert_corrupt(&blob(&[0x30, 0xFB, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0], 1));
    }

    #[test]
    fn bad_headers_are_errors() {
        let good = abc_blob(1, |w| {
            w.text(b"a").end();
        });
        assert_eq!(decompress(&good).unwrap(), b"a");

        let mut bad = good.clone();
        bad[4] = 0x02;
        assert!(matches!(decompress(&bad), Err(Error::BadHeader { .. })));

        let mut refpack = good.clone();
        refpack[16] = 0x10;
        assert!(matches!(decompress(&refpack), Err(Error::BadHeader { .. })));

        let mut size_mismatch = good;
        size_mismatch[8] = 2;
        assert!(matches!(decompress(&size_mismatch), Err(Error::BadHeader { .. })));

        assert_eq!(decompress(b"HUFF"), Err(Error::Truncated(KIND)));
    }

    /// Decompresses every HUFF texture in `CARS/BMWM3GTR/TEXTURES.BIN` and checks
    /// its size and the name hash in the 0x9C-byte texture trailer.
    #[test]
    #[ignore = "needs a game install; set NFSMW_GAME_DIR"]
    fn real_install_bmw_m3_gtr_textures() {
        let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else {
            return;
        };
        let path = std::path::Path::new(&dir).join("CARS/BMWM3GTR/TEXTURES.BIN");
        let file = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let u32_at = |d: &[u8], o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap());

        fn find_chunk(data: &[u8], id: u32) -> Option<&[u8]> {
            let mut pos = 0;
            while let Some(header) = data.get(pos..pos + 8) {
                let cid = u32::from_le_bytes(header[..4].try_into().unwrap());
                let size = u32::from_le_bytes(header[4..].try_into().unwrap()) as usize;
                let payload = data.get(pos + 8..pos + 8 + size)?;
                if cid == id {
                    return Some(payload);
                }
                if cid & 0x8000_0000 != 0
                    && let Some(found) = find_chunk(payload, id)
                {
                    return Some(found);
                }
                pos += 8 + size;
            }
            None
        }

        // The file is a single TexturePack chunk, so entry offsets are file offsets.
        let entries = find_chunk(&file, 0x3331_0003).expect("TexturePackInfoEntries chunk");
        let mut huff = 0;
        for e in entries.as_chunks::<24>().0 {
            let (hash, offset) = (u32_at(e, 0), u32_at(e, 4) as usize);
            let (packed, size) = (u32_at(e, 8) as usize, u32_at(e, 12) as usize);
            let blob = &file[offset..];
            if !blob.starts_with(b"HUFF") {
                continue;
            }
            // The entry's size counts the wrapper header; the wrapper's own field does not.
            assert_eq!(u32_at(blob, 12) as usize + HEADER_LEN, packed, "texture {hash:08X}");
            let out = decompress(blob).unwrap_or_else(|e| panic!("texture {hash:08X}: {e}"));
            assert_eq!(out.len(), size, "texture {hash:08X}");
            assert_eq!(u32_at(&out, size - 0x9C + 0x24), hash, "texture {hash:08X}");
            huff += 1;
        }
        assert!(huff > 0, "no HUFF textures found");
    }
}
