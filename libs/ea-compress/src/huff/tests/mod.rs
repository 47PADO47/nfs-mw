//! HUFF tests on hand-built streams, plus one real-install test.

mod errors;
mod install;
mod streams;

use super::{KIND, decompress};
use crate::Error;

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

fn assert_corrupt(data: &[u8]) {
    let err = decompress(data).unwrap_err();
    assert!(matches!(err, Error::Corrupt { .. }), "{err}");
}
