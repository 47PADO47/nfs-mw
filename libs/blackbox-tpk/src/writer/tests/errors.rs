//! Inputs the writer must refuse.

use super::{bytes, source};
use crate::{PixelFormat, TextureSource, WriteError, write_tpk};

fn texture_error(t: TextureSource) -> String {
    match write_tpk("P", &[t]) {
        Err(WriteError::Texture { detail, .. }) => detail,
        other => panic!("expected a texture error, got {other:?}"),
    }
}

#[test]
fn duplicate_names_are_rejected() {
    let a = source("SAME", 4, 4, PixelFormat::Dxt1, 1, 1);
    let b = source("SAME", 8, 8, PixelFormat::Dxt1, 1, 2);
    match write_tpk("P", &[a, b]) {
        Err(WriteError::Duplicate { first, second, hash }) => {
            assert_eq!((first.as_str(), second.as_str()), ("SAME", "SAME"));
            assert_eq!(hash, blackbox_hash::bstring_hash("SAME"));
        }
        other => panic!("expected a duplicate error, got {other:?}"),
    }
}

#[test]
fn a_hash_override_that_collides_is_a_duplicate() {
    let a = source("A", 4, 4, PixelFormat::Dxt1, 1, 1);
    let mut b = source("B", 4, 4, PixelFormat::Dxt1, 1, 2);
    b.name_hash = Some(a.hash());
    assert!(matches!(write_tpk("P", &[a, b]), Err(WriteError::Duplicate { .. })));
}

#[test]
fn data_length_must_match_the_mips() {
    let mut s = source("SHORT", 8, 8, PixelFormat::Dxt1, 4, 1);
    s.data.pop();
    assert!(texture_error(s).contains("expected 56"));
    let mut long = source("LONG", 8, 8, PixelFormat::Dxt1, 1, 1);
    long.data.push(0);
    assert!(texture_error(long).contains("expected 32"));
}

#[test]
fn dxt_sizes_follow_the_block_size() {
    // 8x8 DXT1 is 4 blocks of 8 bytes; DXT5 would need 64.
    let wrong = TextureSource::new("W", 8, 8, PixelFormat::Dxt5, 1, bytes(1, 32));
    texture_error(wrong);
    let right = TextureSource::new("W", 8, 8, PixelFormat::Dxt5, 1, bytes(1, 64));
    assert!(write_tpk("P", &[right]).is_ok());
}

#[test]
fn mip_count_is_bounded_by_the_size() {
    let mut s = source("MIPS", 4, 4, PixelFormat::Argb8888, 3, 1);
    s.mip_count = 4;
    s.data.extend([0; 4]);
    texture_error(s);
    let mut zero = source("MIPS", 4, 4, PixelFormat::Argb8888, 1, 1);
    zero.mip_count = 0;
    zero.data.clear();
    texture_error(zero);
}

#[test]
fn sizes_must_fit_the_record() {
    texture_error(TextureSource::new("Z", 0, 4, PixelFormat::Dxt1, 1, Vec::new()));
    texture_error(TextureSource::new("Z", 4, 0, PixelFormat::Dxt1, 1, Vec::new()));
    texture_error(TextureSource::new("Z", 0x8000, 4, PixelFormat::Dxt1, 1, Vec::new()));
}

#[test]
fn names_must_be_plain_ascii() {
    texture_error(source("", 4, 4, PixelFormat::Dxt1, 1, 1));
    texture_error(source("CAF\u{c9}", 4, 4, PixelFormat::Dxt1, 1, 1));
    texture_error(source("A\0B", 4, 4, PixelFormat::Dxt1, 1, 1));
}

#[test]
fn unknown_formats_are_rejected() {
    texture_error(TextureSource::new("X", 4, 4, PixelFormat::Other(7), 1, bytes(1, 16)));
}

#[test]
fn palettes_go_with_p8_only() {
    texture_error(source("NOPAL", 4, 4, PixelFormat::P8, 1, 1));
    let mut dxt = source("PAL", 4, 4, PixelFormat::Dxt1, 1, 1);
    dxt.palette = vec![0; 1024];
    texture_error(dxt);
    let mut odd = source("ODDPAL", 4, 4, PixelFormat::P8, 1, 1);
    odd.palette = vec![0; 1022];
    texture_error(odd);
}

#[test]
fn pack_names_must_fit_the_header() {
    let long = "N".repeat(28);
    assert!(matches!(write_tpk(&long, &[]), Err(WriteError::Header(_))));
    assert!(matches!(write_tpk("A\0B", &[]), Err(WriteError::Header(_))));
    assert!(write_tpk(&"N".repeat(27), &[]).is_ok());
}
