//! Malformed streams are rejected with the right error.

use super::{BitWriter, Error, KIND, abc_blob, assert_corrupt, blob, decompress};

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
