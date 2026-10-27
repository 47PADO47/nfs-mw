use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;

use crate::{Bundle, Error, ResourceKind};

fn zlib(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// One resource of type 0x14 with the given blocks. Its import table is at offset 16 of the main block.
fn build(compressed: bool, main: &[u8], graphics: &[u8]) -> Vec<u8> {
    let store = |bytes: &[u8]| if compressed { zlib(bytes) } else { bytes.to_vec() };
    let (main_disk, gfx_disk) = (store(main), store(graphics));
    let main_start = 0x28 + 0x48;
    let gfx_start = main_start + main_disk.len();
    let end = gfx_start + gfx_disk.len();
    let mut out = Vec::new();
    out.extend_from_slice(b"bnd2");
    out.extend_from_slice(&5u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    for value in [end as u32, 1, 0x28, main_start as u32, gfx_start as u32, end as u32, end as u32] {
        push_u32(&mut out, value);
    }
    push_u32(&mut out, u32::from(compressed));
    out.extend_from_slice(&0x0123_4567_89AB_CDEFu64.to_le_bytes());
    let graphics_size = if graphics.is_empty() { 0 } else { graphics.len() as u32 | 0x8000_0000 };
    for size in [main.len() as u32 | 0x4000_0000, graphics_size, 0, 0] {
        push_u32(&mut out, size);
    }
    for size in [main_disk.len() as u32, gfx_disk.len() as u32, 0, 0] {
        push_u32(&mut out, size);
    }
    for _ in 0..4 {
        push_u32(&mut out, 0);
    }
    push_u32(&mut out, 16);
    push_u32(&mut out, 0x14);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    push_u32(&mut out, 0);
    out.extend_from_slice(&main_disk);
    out.extend_from_slice(&gfx_disk);
    out
}

fn main_block(import: u64) -> Vec<u8> {
    let mut main = vec![7u8; 16];
    main.extend_from_slice(&import.to_le_bytes());
    main.extend_from_slice(&4u32.to_le_bytes());
    main.extend_from_slice(&0u32.to_le_bytes());
    main
}

#[test]
fn reads_plain_and_compressed_bundles_alike() {
    let import = 0x0100_0000_0000_ABCD;
    let main = main_block(import);
    let graphics: Vec<u8> = (0..=255).collect();
    for compressed in [false, true] {
        let file = build(compressed, &main, &graphics);
        let bundle = Bundle::parse(&file).unwrap();
        assert_eq!(bundle.entries().len(), 1);
        let entry = bundle.entries()[0];
        assert_eq!(entry.id, 0x0123_4567_89AB_CDEF);
        assert_eq!(entry.block_alignment(0), 16);
        assert_eq!(entry.block_alignment(1), 256);
        assert_eq!(ResourceKind::from_id(entry.kind), Some(ResourceKind::GenesysType));
        let resource = bundle.resource_by_id(entry.id).unwrap();
        assert_eq!(resource.blocks[0], main);
        assert_eq!(resource.blocks[1], graphics);
        assert!(resource.blocks[2].is_empty());
        assert_eq!(resource.imports.len(), 1);
        assert_eq!((resource.imports[0].id, resource.imports[0].offset), (import, 4));
    }
}

#[test]
fn refuses_what_it_cannot_read() {
    assert!(matches!(Bundle::parse(b"nope"), Err(Error::Truncated { .. })));
    let mut file = build(false, &main_block(1), &[]);
    file[0] = b'x';
    assert!(matches!(Bundle::parse(&file), Err(Error::BadMagic { .. })));
    let mut file = build(false, &main_block(1), &[]);
    file[4] = 6;
    assert!(matches!(Bundle::parse(&file), Err(Error::Unsupported { version: 6, platform: 1 })));
    let file = build(false, &main_block(1), &[]);
    let bundle = Bundle::parse(&file).unwrap();
    assert!(matches!(bundle.resource_by_id(5), Err(Error::NoSuchResource { id: 5 })));
}

#[test]
fn a_damaged_stream_is_an_error_not_a_panic() {
    let mut file = build(true, &main_block(1), &[1, 2, 3, 4]);
    let last = file.len() - 1;
    file[last - 3] ^= 0xFF;
    file[last - 4] ^= 0xFF;
    let bundle = Bundle::parse(&file).unwrap();
    assert!(bundle.resource(0).is_err());
}
