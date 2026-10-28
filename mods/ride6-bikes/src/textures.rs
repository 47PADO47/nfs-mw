//! The textures of a bike's `TEXTURES.BIN`: the pictures of the glTF files cut down to a size limit and stored as 32-bit
//! maps with their mip levels, and flat colours for the surfaces that have a colour and no picture.

use std::collections::HashMap;
use std::io::Cursor;

use anyhow::{Context, Result, ensure};
use blackbox_tpk::{AlphaUsage, PixelFormat, TextureSource};

/// The textures one bike adds to its pack, by name.
#[derive(Default)]
pub struct Palette {
    textures: Vec<TextureSource>,
    names: HashMap<String, usize>,
}

impl Palette {
    pub fn into_textures(self) -> Vec<TextureSource> {
        self.textures
    }

    /// Add the PNG `png` under `name` (once), no larger than `max_size` on its longer side. The name is the same on a
    /// second call, so two parts that share a picture share the texture.
    pub fn add_png(&mut self, name: &str, png: &[u8], max_size: u32) -> Result<String> {
        if self.names.contains_key(name) {
            return Ok(name.to_owned());
        }
        let picture = decode(png).with_context(|| format!("decoding the picture {name}"))?;
        let source = encode(name, picture, max_size);
        self.names.insert(name.to_owned(), self.textures.len());
        self.textures.push(source);
        Ok(name.to_owned())
    }

    /// Add a flat colour (sRGB bytes) as a texture (once), and return its name.
    pub fn add_colour(&mut self, rgb: [u8; 3]) -> String {
        let [r, g, b] = rgb;
        let name = format!("C_{r:02X}{g:02X}{b:02X}");
        if self.names.contains_key(&name) {
            return name;
        }
        self.names.insert(name.clone(), self.textures.len());
        self.textures.push(TextureSource::new(&name, 4, 4, PixelFormat::Dxt1, 1, solid_block(r, g, b)));
        name
    }
}

/// One DXT1 block of a single colour: both end points the same, every texel using the first.
fn solid_block(r: u8, g: u8, b: u8) -> Vec<u8> {
    let rgb565 = (u16::from(r >> 3) << 11) | (u16::from(g >> 2) << 5) | u16::from(b >> 3);
    let end = rgb565.to_le_bytes();
    vec![end[0], end[1], end[0], end[1], 0, 0, 0, 0]
}

/// A picture as 8-bit RGBA rows.
struct Picture {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn decode(png: &[u8]) -> Result<Picture> {
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

/// Halve a picture by averaging 2 by 2 blocks (an edge pixel repeats).
fn halve(picture: &Picture) -> Picture {
    let (width, height) = ((picture.width / 2).max(1), (picture.height / 2).max(1));
    let at = |x: u32, y: u32, channel: usize| {
        let (x, y) = (x.min(picture.width - 1), y.min(picture.height - 1));
        u32::from(picture.rgba[((y * picture.width + x) * 4) as usize + channel])
    };
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            for channel in 0..4 {
                let sum = at(2 * x, 2 * y, channel)
                    + at(2 * x + 1, 2 * y, channel)
                    + at(2 * x, 2 * y + 1, channel)
                    + at(2 * x + 1, 2 * y + 1, channel);
                rgba.push(((sum + 2) / 4) as u8);
            }
        }
    }
    Picture { width, height, rgba }
}

/// The picture, cut to `max_size`, as a 32-bit texture with a full mip chain. The file stores the bytes of a pixel as
/// blue, green, red, alpha.
fn encode(name: &str, mut picture: Picture, max_size: u32) -> TextureSource {
    while picture.width.max(picture.height) > max_size {
        picture = halve(&picture);
    }
    let (width, height) = (picture.width, picture.height);
    let mips = width.max(height).ilog2() + 1;
    let mut data = Vec::new();
    let mut level = picture;
    for n in 0..mips {
        data.extend(level.rgba.as_chunks::<4>().0.iter().flat_map(|&[r, g, b, a]| [b, g, r, a]));
        if n + 1 < mips {
            level = halve(&level);
        }
    }
    let mut source = TextureSource::new(name, width, height, PixelFormat::Argb8888, mips, data);
    source.alpha_usage = AlphaUsage::None;
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(width: u32, height: u32) -> Picture {
        Picture { width, height, rgba: (0..width * height * 4).map(|n| (n % 251) as u8).collect() }
    }

    #[test]
    fn large_pictures_are_cut_to_the_limit_and_keep_every_mip() {
        let source = encode("T", picture(256, 128), 64);
        assert_eq!((source.width, source.height), (64, 32));
        assert_eq!(source.mip_count, 7);
        assert_eq!(source.data.len(), source.expected_data_len());
    }

    #[test]
    fn halving_averages_blocks() {
        let one = Picture {
            width: 2,
            height: 2,
            rgba: vec![0, 0, 0, 0, 100, 100, 100, 100, 100, 100, 100, 100, 200, 200, 200, 200],
        };
        assert_eq!(halve(&one).rgba, vec![100, 100, 100, 100]);
    }

    #[test]
    fn a_colour_makes_one_texture() {
        let mut palette = Palette::default();
        let first = palette.add_colour([1, 2, 3]);
        assert_eq!(first, palette.add_colour([1, 2, 3]));
        assert_eq!(first, "C_010203");
        assert_eq!(palette.into_textures().len(), 1);
    }

    #[test]
    fn a_png_round_trips_to_a_texture() {
        let mut png_bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut png_bytes, 4, 4);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&[255; 64]).unwrap();
        let mut palette = Palette::default();
        palette.add_png("T", &png_bytes, 16).unwrap();
        let textures = palette.into_textures();
        assert_eq!((textures[0].width, textures[0].mip_count), (4, 3));
    }
}
