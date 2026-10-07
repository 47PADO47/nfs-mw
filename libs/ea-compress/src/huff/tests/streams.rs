//! Valid streams decode to the expected bytes.

use super::{BitWriter, abc_blob, blob, decompress};

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
