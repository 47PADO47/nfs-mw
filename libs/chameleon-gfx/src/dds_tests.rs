//! `Texture::to_dds` against the DDS layout, on synthetic textures and (with `CHAMELEON_GAME_DIR`) the car textures.

use chameleon_bundle::{Bundle, ResourceKind};

use crate::{Error, PixelFormat, Texture};

fn word(dds: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([dds[offset], dds[offset + 1], dds[offset + 2], dds[offset + 3]])
}

fn texture(dxgi_format: u32, width: u32, height: u32, mip_levels: u32, array_size: u32) -> Texture {
    let format = PixelFormat::from_dxgi(dxgi_format);
    let mut texture =
        Texture { width, height, depth: 1, array_size, mip_levels, dxgi_format, format, data: Vec::new() };
    let size = texture.slice_size().unwrap_or(64) * array_size as usize;
    texture.data = (0..size).map(|n| n as u8).collect();
    texture
}

#[test]
fn a_bc1_texture_gets_the_classic_header() {
    let texture = texture(71, 16, 8, 3, 1);
    let dds = texture.to_dds().unwrap();
    // Magic, 124-byte header, no DX10 extension, then the pixels untouched.
    assert_eq!(&dds[..4], b"DDS ");
    assert_eq!(dds.len(), 128 + texture.data.len());
    assert_eq!(&dds[128..], &texture.data[..]);
    assert_eq!(word(&dds, 4), 124);
    assert_eq!(word(&dds, 8), 0x1 | 0x2 | 0x4 | 0x1000 | 0x2_0000 | 0x8_0000);
    assert_eq!((word(&dds, 12), word(&dds, 16)), (8, 16), "height then width");
    assert_eq!(word(&dds, 20), 4 * 2 * 8, "top mip: 4 x 2 blocks of 8 bytes");
    assert_eq!(word(&dds, 28), 3);
    assert_eq!((word(&dds, 76), word(&dds, 80), &dds[84..88]), (32, 0x4, &b"DXT1"[..]));
    assert_eq!(word(&dds, 108), 0x1000 | 0x8 | 0x40_0000);
}

#[test]
fn bc2_and_bc3_use_their_classic_codes_and_one_mip_has_no_mip_flags() {
    for (dxgi, code) in [(74, b"DXT3"), (77, b"DXT5")] {
        let dds = texture(dxgi, 8, 8, 1, 1).to_dds().unwrap();
        assert_eq!(&dds[84..88], code);
        assert_eq!(word(&dds, 8) & 0x2_0000, 0);
        assert_eq!(word(&dds, 108), 0x1000);
    }
}

#[test]
fn other_formats_and_arrays_get_the_dx10_extension() {
    for (dxgi, width, slices) in [(72, 8, 1), (77, 8, 2), (98, 8, 1), (28, 4, 1)] {
        let texture = texture(dxgi, width, 8, 2, slices);
        let dds = texture.to_dds().unwrap();
        assert_eq!(&dds[84..88], b"DX10", "dxgi {dxgi}, {slices} slices");
        assert_eq!(dds.len(), 128 + 20 + texture.data.len());
        assert_eq!((word(&dds, 128), word(&dds, 132), word(&dds, 140)), (dxgi, 3, slices));
    }
    // An uncompressed format records the row pitch, not a linear size.
    let rgba = texture(28, 4, 8, 1, 1).to_dds().unwrap();
    assert_eq!((word(&rgba, 8) & 0x8, word(&rgba, 8) & 0x8_0000, word(&rgba, 20)), (0x8, 0, 16));
}

#[test]
fn an_unknown_format_keeps_its_bytes_and_the_dxgi_number() {
    let mut odd = texture(71, 8, 8, 1, 1);
    odd.dxgi_format = 1234;
    odd.format = PixelFormat::from_dxgi(1234);
    let dds = odd.to_dds().unwrap();
    assert_eq!((word(&dds, 128), word(&dds, 20), word(&dds, 8) & 0x8_0008), (1234, 0, 0));
    assert_eq!(&dds[148..], &odd.data[..]);
}

#[test]
fn refuses_what_it_cannot_describe() {
    let mut volume = texture(71, 8, 8, 1, 1);
    volume.depth = 4;
    assert!(matches!(volume.to_dds(), Err(Error::Invalid { what: "dds", .. })));
    let mut short = texture(71, 16, 16, 5, 1);
    short.data.truncate(short.data.len() - 1);
    assert!(matches!(short.to_dds(), Err(Error::Invalid { what: "dds", .. })));
    // Padding after the last mip is dropped, not written.
    let mut padded = texture(71, 8, 8, 1, 1);
    padded.data.extend_from_slice(&[9; 32]);
    assert_eq!(padded.to_dds().unwrap().len(), 128 + 32);
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_shared_car_textures_export() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let data = std::fs::read(std::path::Path::new(&dir).join("VEHICLES/VEHICLETEX.BNDL")).unwrap();
    let bundle = Bundle::parse(&data).unwrap();
    let mut exported = 0;
    for index in 0..bundle.entries().len() {
        let resource = bundle.resource(index).unwrap();
        if resource.entry.kind != ResourceKind::Texture.id() {
            continue;
        }
        let texture = Texture::parse(&resource.blocks[0], &resource.blocks[1]).unwrap();
        let dds = texture.to_dds().unwrap();
        assert_eq!(&dds[..4], b"DDS ");
        assert_eq!(
            (word(&dds, 12), word(&dds, 16), word(&dds, 28)),
            (texture.height, texture.width, texture.mip_levels)
        );
        assert_eq!(&dds[84..88], if texture.dxgi_format == 71 { b"DXT1" } else { b"DXT5" });
        // The graphics block is the pixels of all mips rounded up to a multiple of 128 bytes; the padding is not written.
        let pixels = texture.slice_size().unwrap();
        assert_eq!(texture.data.len(), pixels.next_multiple_of(128));
        assert_eq!(dds.len(), 128 + pixels);
        exported += 1;
    }
    assert_eq!(exported, 53);
}
