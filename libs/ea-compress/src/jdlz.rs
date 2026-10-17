//! JDLZ: EA's LZ77 variant with two independent flag-bit streams.
//!
//! Implemented from our own spec, `docs/formats/bchunk.md` §3.

use crate::{Error, HEADER_LEN, Header, JDLZ_MAGIC, Result};

const KIND: &str = "JDLZ";

fn corrupt(input: usize, output: usize, detail: &'static str) -> Error {
    Error::Corrupt { kind: KIND, input, output, detail }
}

/// Append `length` bytes copied from `out[start..]`, LZ77 style.
///
/// Semantically this is the byte-by-byte loop `for i in 0..length { out.push(out[start + i]) }`:
/// byte `k` of the copy is read after the first `k` copied bytes exist, so an overlapping copy
/// (`distance < length`, with `distance = out.len() - start`) repeats the last `distance` bytes.
///
/// Speed-up: each step appends `out[start..start + n]` with `n <= out.len() - start`, so the
/// source span never reads past what exists. Let `d` be the distance and `P(x)` the byte the
/// reference loop produces at `x`. Every byte appended so far satisfies `P(x + d) == P(x)`
/// for `x >= start`, so `P` is periodic with period `d`. The span `out.len() - start` starts at
/// `d` and then doubles after each full step, so it is always a multiple of `d`. Appending
/// `out[start..start + n]` at position `out.len()` therefore writes `P(start + k)` at
/// `out.len() + k`, which is exactly what the reference loop writes there. The only partial
/// step is the last one, which ends the copy. Precondition: `start < out.len()` when `length > 0`.
fn copy_back(out: &mut Vec<u8>, start: usize, length: usize) {
    debug_assert!(length == 0 || start < out.len());
    let distance = out.len() - start;
    if distance >= length {
        // Non-overlapping: a single copy from already-final bytes.
        out.extend_from_within(start..start + length);
        return;
    }
    let mut remaining = length;
    while remaining > 0 {
        let n = remaining.min(out.len() - start);
        out.extend_from_within(start..start + n);
        remaining -= n;
    }
}

/// Decompress a JDLZ blob (16-byte header followed by the LZ stream).
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let h = Header::parse(data, KIND)?;
    if h.magic != JDLZ_MAGIC || h.version != 0x02 || h.flags != 0x10 {
        return Err(Error::BadHeader {
            kind: KIND,
            detail: format!("magic/version/flags {:08X}/{:02X}/{:02X}", h.magic, h.version, h.flags),
        });
    }
    let out_len = h.decompressed_size as usize;
    let end = (h.compressed_size as usize).min(data.len());
    let data = &data[..end];
    let mut out = Vec::with_capacity(out_len);
    let mut ip = HEADER_LEN;
    let (mut flags1, mut flags2) = (1u32, 1u32);

    while ip < end && out.len() < out_len {
        if flags1 == 1 {
            let b = *data.get(ip).ok_or_else(|| corrupt(ip, out.len(), "flag byte past end"))?;
            flags1 = u32::from(b) | 0x100;
            ip += 1;
        }
        if flags2 == 1 {
            let b = *data.get(ip).ok_or_else(|| corrupt(ip, out.len(), "flag byte past end"))?;
            flags2 = u32::from(b) | 0x100;
            ip += 1;
        }
        if flags1 & 1 != 0 {
            let (b0, b1) = match data.get(ip..ip + 2) {
                Some(&[b0, b1]) => (usize::from(b0), usize::from(b1)),
                _ => return Err(corrupt(ip, out.len(), "back-reference past end")),
            };
            ip += 2;
            let (length, distance) = if flags2 & 1 != 0 {
                ((((b0 & 0xF0) << 4) | b1) + 3, (b0 & 0x0F) + 1)
            } else {
                ((b0 & 0x1F) + 3, (((b0 & 0xE0) << 3) | b1) + 17)
            };
            flags2 >>= 1;
            let start =
                out.len().checked_sub(distance).ok_or_else(|| corrupt(ip, out.len(), "back-reference before start"))?;
            let length = length.min(out_len - out.len());
            copy_back(&mut out, start, length);
        } else {
            let b = *data.get(ip).ok_or_else(|| corrupt(ip, out.len(), "literal past end"))?;
            out.push(b);
            ip += 1;
        }
        flags1 >>= 1;
    }

    if out.len() != out_len {
        return Err(corrupt(ip, out.len(), "stream ended before the decompressed size"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(stream: &[u8], out_size: u32) -> Vec<u8> {
        let mut v = b"JDLZ".to_vec();
        v.extend_from_slice(&[0x02, 0x10, 0, 0]);
        v.extend_from_slice(&out_size.to_le_bytes());
        v.extend_from_slice(&(16 + stream.len() as u32).to_le_bytes());
        v.extend_from_slice(stream);
        v
    }

    #[test]
    fn literals_only() {
        let data = blob(&[0x00, 0x00, b'a', b'b', b'c', b'd'], 4);
        assert_eq!(decompress(&data).unwrap(), b"abcd");
    }

    #[test]
    fn short_overlapping_backref() {
        // Literal 'x', then a short-form back-reference: distance 1, length 5.
        // flags1 = 0b10 (literal, backref); flags2 bit 0 = 1 (short form).
        let data = blob(&[0b10, 0b1, b'x', 0x00, 0x02], 6);
        assert_eq!(decompress(&data).unwrap(), b"xxxxxx");
    }

    #[test]
    fn long_backref() {
        // 17 literals, then a long-form back-reference: distance 17, length 3.
        let mut stream = vec![0x00, 0x00];
        stream.extend(b'a'..=b'h');
        stream.push(0x00);
        stream.extend(b'i'..=b'p');
        stream.push(0b10);
        stream.push(b'q');
        stream.extend_from_slice(&[0x00, 0x00]);
        let data = blob(&stream, 20);
        assert_eq!(decompress(&data).unwrap(), b"abcdefghijklmnopqabc");
    }

    #[test]
    fn backref_before_start_is_error() {
        let data = blob(&[0b1, 0b1, 0x00, 0x00], 3);
        assert!(matches!(decompress(&data), Err(Error::Corrupt { .. })));
    }

    #[test]
    fn truncated_stream_is_error() {
        let data = blob(&[0x00, 0x00, b'a'], 4);
        assert!(decompress(&data).is_err());
    }

    /// The original byte-by-byte copy, kept as the reference for `copy_back`.
    fn copy_back_reference(out: &mut Vec<u8>, start: usize, length: usize) {
        for i in 0..length {
            let b = out[start + i];
            out.push(b);
        }
    }

    /// Minimal seeded xorshift64 so the tests stay deterministic without a new dependency.
    struct XorShift(u64);

    impl XorShift {
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
    }

    #[test]
    fn copy_back_matches_byte_by_byte_reference() {
        let mut rng = XorShift(0x9E37_79B9_7F4A_7C15);
        for distance in 1..=40usize {
            for length in 1..=100usize {
                let prefix_len = distance + (rng.next_u64() % 64) as usize;
                let prefix: Vec<u8> = (0..prefix_len).map(|_| rng.next_u64() as u8).collect();
                let start = prefix_len - distance;

                let mut expected = prefix.clone();
                copy_back_reference(&mut expected, start, length);
                let mut actual = prefix;
                copy_back(&mut actual, start, length);

                assert_eq!(actual, expected, "distance {distance}, length {length}");
            }
        }
    }

    #[test]
    fn copy_back_zero_length_is_noop() {
        let mut out = b"abc".to_vec();
        copy_back(&mut out, 0, 0);
        assert_eq!(out, b"abc");
    }
}
