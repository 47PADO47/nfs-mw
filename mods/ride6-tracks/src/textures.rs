//! The pictures of a world: PNG files of the pack's `tex/` folder, shrunk and stored as DXT1 with a full mip chain, the form
//! the game's own ground textures have. A material with no picture gets a one-block flat colour.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use blackbox_tpk::{Level, PixelFormat, TextureSource, full_mip_count, halve};

/// A decoded picture as 8-bit RGBA rows.
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Decodes a PNG of any 8-bit colour type.
pub fn decode(png: &[u8]) -> Result<Picture> {
    let mut decoder = png::Decoder::new(Cursor::new(png));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![0; reader.output_buffer_size().context("the picture is too large")?];
    let info = reader.next_frame(&mut buffer)?;
    ensure!(info.bit_depth == png::BitDepth::Eight, "only 8-bit pictures are read");
    let pixels = &buffer[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => pixels.as_chunks::<3>().0.iter().flat_map(|&[r, g, b]| [r, g, b, 255]).collect(),
        png::ColorType::GrayscaleAlpha => pixels.as_chunks::<2>().0.iter().flat_map(|&[g, a]| [g, g, g, a]).collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => anyhow::bail!("an indexed picture was not expanded"),
    };
    Ok(Picture { width: info.width, height: info.height, rgba })
}

/// The texture a picture becomes: halved until its longer side is at most `limit`, as DXT1 with all its mip levels. The
/// name is the texture's name in the game (the solids refer to its hash).
pub fn texture_of(name: &str, picture: Picture, limit: u32) -> Result<TextureSource> {
    ensure!(picture.width > 0 && picture.height > 0, "{name} is an empty picture");
    let mut level = Level { width: picture.width, height: picture.height, rgba: picture.rgba };
    while level.width.max(level.height) > limit.max(1) {
        level = halve(&level);
    }
    let mips = full_mip_count(level.width, level.height);
    let (width, height) = (level.width, level.height);
    TextureSource::from_rgba(name, level, PixelFormat::Dxt1, mips)
        .with_context(|| format!("{name}: {width} by {height} pixels cannot be stored"))
}

/// The game's name of a picture: its file stem in upper case, which is also what the manifest's solids hash.
pub fn game_name(stem: &str) -> String {
    stem.to_ascii_uppercase()
}

/// The name of a flat-colour texture.
pub fn colour_name([r, g, b]: [u8; 3]) -> String {
    format!("C_{r:02X}{g:02X}{b:02X}")
}

/// A 4 by 4 texture of one colour: both end points the same, every texel using the first.
pub fn flat_colour([r, g, b]: [u8; 3]) -> TextureSource {
    let rgb565 = (u16::from(r >> 3) << 11) | (u16::from(g >> 2) << 5) | u16::from(b >> 3);
    let end = rgb565.to_le_bytes();
    let block = vec![end[0], end[1], end[0], end[1], 0, 0, 0, 0];
    TextureSource::new(colour_name([r, g, b]), 4, 4, PixelFormat::Dxt1, 1, block)
}

/// The pictures of a pack's `tex/` folder, converted once each.
pub struct Pictures {
    textures: HashMap<String, TextureSource>,
}

impl Pictures {
    /// Converts the pictures named by `stems` (lower case file stems) from `dir`, `limit` pixels at most. A picture that is
    /// missing or cannot be read is left out, with the reason in the list that comes back.
    pub fn load(dir: &Path, stems: &[String], limit: u32) -> (Self, Vec<(String, String)>) {
        let workers = std::thread::available_parallelism().map_or(2, |n| n.get().clamp(1, 6));
        let chunk = stems.len().div_ceil(workers).max(1);
        let results: Vec<(String, Result<TextureSource>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = stems
                .chunks(chunk)
                .map(|part| {
                    scope.spawn(move || {
                        part.iter().map(|stem| (stem.clone(), convert(dir, stem, limit))).collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|handle| handle.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic)))
                .collect()
        });
        let mut textures = HashMap::new();
        let mut failed = Vec::new();
        for (stem, result) in results {
            match result {
                Ok(texture) => {
                    textures.insert(stem, texture);
                }
                Err(error) => failed.push((stem, format!("{error:#}"))),
            }
        }
        (Self { textures }, failed)
    }

    /// The texture of a picture stem, if it was converted.
    pub fn get(&self, stem: &str) -> Option<&TextureSource> {
        self.textures.get(stem)
    }
}

fn convert(dir: &Path, stem: &str, limit: u32) -> Result<TextureSource> {
    let path = dir.join(format!("{stem}.png"));
    let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let picture = decode(&bytes).with_context(|| format!("decoding {}", path.display()))?;
    texture_of(&game_name(stem), picture, limit)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn png_bytes(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(rgba).unwrap();
        out
    }

    #[test]
    fn a_picture_is_shrunk_to_the_limit_and_keeps_a_full_mip_chain() {
        let picture = decode(&png_bytes(64, 32, &vec![200; 64 * 32 * 4])).unwrap();
        let texture = texture_of("T_TEST", picture, 16).unwrap();
        assert_eq!((texture.width, texture.height), (16, 8));
        assert_eq!(texture.mip_count, full_mip_count(16, 8));
        assert_eq!(texture.format, PixelFormat::Dxt1);
    }

    #[test]
    fn a_flat_colour_is_one_block_named_by_its_colour() {
        let texture = flat_colour([1, 2, 255]);
        assert_eq!(texture.name, "C_0102FF");
        assert_eq!(texture.data.len(), 8);
    }

    #[test]
    fn pictures_are_converted_from_a_folder_and_a_missing_one_is_reported() {
        let dir = std::env::temp_dir().join(format!("ride6-tracks-tex-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("t_grass.png"), png_bytes(8, 8, &vec![90; 8 * 8 * 4])).unwrap();
        let stems = vec!["t_grass".to_owned(), "t_gone".to_owned()];
        let (pictures, failed) = Pictures::load(&dir, &stems, 512);
        assert_eq!(pictures.get("t_grass").map(|t| t.name.as_str()), Some("T_GRASS"));
        assert!(pictures.get("t_gone").is_none());
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].0, "t_gone");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
