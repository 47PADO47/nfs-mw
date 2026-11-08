//! Textures of a replacement set on authored pictures: no game data.

use blackbox_modding::content::{TextureConvert, TextureState};
use blackbox_tpk::{AlphaUsage, PixelFormat, Texture, TextureSource};

use super::*;

/// A PNG of `width` by `height` pixels, with every pixel `rgba`.
fn png_of(width: u32, height: u32, color: png::ColorType, pixel: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(color);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    let data: Vec<u8> = pixel.iter().copied().cycle().take((width * height) as usize * pixel.len()).collect();
    writer.write_image_data(&data).unwrap();
    writer.finish().unwrap();
    out
}

fn stock(name: &str) -> Texture {
    let mut source = TextureSource::new(name, 8, 8, PixelFormat::Dxt1, 1, vec![0; 32]);
    source.alpha_usage = AlphaUsage::PunchThrough;
    source.alpha_blend = 1;
    source.flags = 2;
    source.tilable_uv = 3;
    source.into_texture()
}

#[test]
fn an_opaque_png_becomes_bc1_with_a_full_mip_chain_and_one_with_alpha_bc3() {
    let opaque =
        picture("png", "T", &png_of(16, 8, png::ColorType::Rgb, &[200, 100, 50]), TextureConvert::Auto, None).unwrap();
    assert_eq!((opaque.format, opaque.width, opaque.height, opaque.mip_levels), (PixelFormat::Dxt1, 16, 8, 5));
    assert_eq!(opaque.alpha_usage, AlphaUsage::None);
    let alpha =
        picture("png", "T", &png_of(8, 8, png::ColorType::Rgba, &[1, 2, 3, 100]), TextureConvert::Auto, None).unwrap();
    assert_eq!((alpha.format, alpha.alpha_usage), (PixelFormat::Dxt5, AlphaUsage::Modulated));
    let grey =
        picture("png", "T", &png_of(4, 4, png::ColorType::Grayscale, &[90]), TextureConvert::Auto, None).unwrap();
    assert_eq!(grey.format, PixelFormat::Dxt1);
}

#[test]
fn a_size_that_is_not_whole_blocks_is_stored_uncompressed_and_a_forced_block_format_refuses_it() {
    let bytes = png_of(6, 5, png::ColorType::Rgba, &[9, 9, 9, 255]);
    let plain = picture("png", "T", &bytes, TextureConvert::Auto, None).unwrap();
    assert_eq!((plain.format, plain.width, plain.height), (PixelFormat::Argb8888, 6, 5));
    assert_eq!(plain.mip_levels, 3, "6 by 5, 3 by 2, 1 by 1");
    assert_eq!(plain.data.len(), (30 + 6 + 1) * 4, "pixels of four bytes");
    assert!(picture("png", "T", &bytes, TextureConvert::Bc3, None).is_err());
    let forced =
        picture("png", "T", &png_of(8, 8, png::ColorType::Rgba, &[1, 2, 3, 255]), TextureConvert::Rgba8, None).unwrap();
    assert_eq!(forced.format, PixelFormat::Argb8888);
}

#[test]
fn a_cap_halves_a_picture_until_it_fits() {
    let big = png_of(64, 32, png::ColorType::Rgb, &[1, 2, 3]);
    let capped = picture("png", "T", &big, TextureConvert::Auto, Some(16)).unwrap();
    assert_eq!((capped.width, capped.height), (16, 8));
}

#[test]
fn a_file_that_is_not_a_picture_is_refused() {
    assert!(picture("png", "T", b"not a png", TextureConvert::Auto, None).is_err());
    assert!(picture("jpg", "T", b"", TextureConvert::Auto, None).is_err());
    assert!(picture("dds", "T", b"short", TextureConvert::Auto, None).is_err());
}

#[test]
fn a_replacement_keeps_the_name_hash_and_by_default_the_draw_state_of_the_original() {
    let picture = TextureSource::new("MODS_OWN", 16, 16, PixelFormat::Dxt5, 1, vec![0; 256]).into_texture();
    let original = stock("ROAD_TEX");
    let kept = TextureSlot::ready("m", "ROAD_TEX", picture.clone(), TextureState::Keep).replacement(&original).unwrap();
    assert_eq!((kept.name.as_str(), kept.name_hash), ("ROAD_TEX", original.name_hash));
    assert_eq!((kept.width, kept.format), (16, PixelFormat::Dxt5), "the pixels and the size are the picture's");
    assert_eq!((kept.alpha_usage, kept.alpha_blend, kept.flags, kept.tilable_uv), (AlphaUsage::PunchThrough, 1, 2, 3));
    let own = TextureSlot::ready("m", "ROAD_TEX", picture, TextureState::File).replacement(&original).unwrap();
    assert_eq!((own.alpha_usage, own.flags), (AlphaUsage::None, 0), "the file's own state");
    assert_eq!(own.name_hash, original.name_hash, "the hash is the original's either way");
}

#[test]
fn a_cap_drops_top_mips_of_a_texture_that_has_them() {
    let mut texture = TextureSource::new("T", 16, 16, PixelFormat::Dxt1, 3, vec![1; 128 + 32 + 8]).into_texture();
    shrink(&mut texture, 8);
    assert_eq!((texture.width, texture.mip_levels, texture.data.len()), (8, 2, 32 + 8));
    let mut single = TextureSource::new("T", 16, 16, PixelFormat::Dxt1, 1, vec![1; 128]).into_texture();
    shrink(&mut single, 4);
    assert_eq!((single.width, single.mip_levels), (16, 1), "a texture with one level cannot be shrunk");
}

fn folder(name: &str, files: &[(&str, &[u8])]) -> (std::path::PathBuf, GameDir) {
    let root = std::env::temp_dir().join(format!("nfsmw-textures-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (path, bytes) in files {
        let full = root.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, bytes).unwrap();
    }
    let dir = GameDir::open(&root).unwrap();
    (root, dir)
}

#[test]
fn a_file_is_read_once_when_first_needed_and_a_missing_one_leaves_the_original() {
    let png = png_of(8, 8, png::ColorType::Rgb, &[5, 6, 7]);
    let (root, dir) = folder("lazy", &[("TEX/A.png", &png)]);
    let slot =
        TextureSlot::file("m", "ROAD_TEX", "TEX/A.png", dir.clone(), TextureState::Keep, TextureConvert::Auto, None);
    let original = stock("ROAD_TEX");
    let first = slot.replacement(&original).unwrap();
    assert_eq!((first.width, first.format, first.flags), (8, PixelFormat::Dxt1, 2));
    std::fs::remove_file(root.join("TEX/A.png")).unwrap();
    assert_eq!(slot.replacement(&original).unwrap(), first, "kept: one read serves every section");
    let missing = TextureSlot::file("m", "T", "TEX/NONE.png", dir, TextureState::Keep, TextureConvert::Auto, None);
    assert!(missing.replacement(&original).is_none(), "the game's texture stays");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_texture_of_a_bin_is_found_by_name_or_as_the_only_one() {
    let pack = blackbox_tpk::write_tpk("P", &[TextureSource::new("ONLY_ONE", 4, 4, PixelFormat::Dxt1, 1, vec![0; 8])])
        .unwrap();
    assert_eq!(picture("bin", "WHATEVER", &pack, TextureConvert::Auto, None).unwrap().name, "ONLY_ONE");
    let two = blackbox_tpk::write_tpk(
        "P",
        &[
            TextureSource::new("A", 4, 4, PixelFormat::Dxt1, 1, vec![0; 8]),
            TextureSource::new("B", 4, 4, PixelFormat::Dxt1, 1, vec![0; 8]),
        ],
    )
    .unwrap();
    assert_eq!(picture("bin", "B", &two, TextureConvert::Auto, None).unwrap().name, "B");
    assert!(picture("bin", "C", &two, TextureConvert::Auto, None).is_err());
}
