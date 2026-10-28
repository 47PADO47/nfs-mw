//! The pictures of a bike: PNG files from the exporter's `tex` folder, and the textures of the car's `TEXTURES.BIN`.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Context, Result, ensure};
use blackbox_tpk::{Level, PixelFormat, TextureSource, halve};

/// A picture as 8-bit RGBA rows.
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Picture {
    /// The texel under `uv` (wrapped, nearest), as RGBA. The row 0 of a picture is v = 0.
    pub fn sample(&self, [u, v]: [f32; 2]) -> [u8; 4] {
        let wrap = |t: f32| t - t.floor();
        let x = ((wrap(u) * self.width as f32) as u32).min(self.width - 1);
        let y = ((wrap(v) * self.height as f32) as u32).min(self.height - 1);
        let at = ((y * self.width + x) * 4) as usize;
        [self.rgba[at], self.rgba[at + 1], self.rgba[at + 2], self.rgba[at + 3]]
    }

    /// The picture halved until its longer side is at most `limit`.
    pub fn shrunk(self, limit: u32) -> Level {
        let mut level = Level { width: self.width, height: self.height, rgba: self.rgba };
        while level.width.max(level.height) > limit.max(1) {
            level = halve(&level);
        }
        level
    }
}

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

/// The exporter's `tex` folder: pictures by file stem, read once.
pub struct TexStore {
    files: HashMap<String, std::path::PathBuf>,
    loaded: HashMap<String, Rc<Picture>>,
}

impl TexStore {
    pub fn open(dir: &Path) -> Result<Self> {
        let mut files = HashMap::new();
        for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
            let path = entry?.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()).map(str::to_ascii_lowercase) else { continue };
            if let Some(stem) = name.strip_suffix(".png") {
                files.insert(stem.to_owned(), path);
            }
        }
        Ok(Self { files, loaded: HashMap::new() })
    }

    /// The picture with this file stem (lower case), or `None` when the exporter wrote none.
    pub fn picture(&mut self, stem: &str) -> Result<Option<Rc<Picture>>> {
        if let Some(done) = self.loaded.get(stem) {
            return Ok(Some(done.clone()));
        }
        let Some(path) = self.files.get(stem) else { return Ok(None) };
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let picture = Rc::new(decode(&bytes).with_context(|| format!("decoding {}", path.display()))?);
        self.loaded.insert(stem.to_owned(), picture.clone());
        Ok(Some(picture))
    }
}

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

    pub fn contains(&self, name: &str) -> bool {
        self.names.contains_key(name)
    }

    /// Add a finished texture (once per name).
    pub fn add(&mut self, source: TextureSource) {
        if self.names.contains_key(&source.name) {
            return;
        }
        self.names.insert(source.name.clone(), self.textures.len());
        self.textures.push(source);
    }

    /// Add a flat colour (sRGB bytes) as a texture (once), and return its name.
    pub fn add_colour(&mut self, rgb: [u8; 3]) -> String {
        let [r, g, b] = rgb;
        let name = format!("C_{r:02X}{g:02X}{b:02X}");
        if !self.names.contains_key(&name) {
            self.add(TextureSource::new(&name, 4, 4, PixelFormat::Dxt1, 1, solid_block(r, g, b)));
        }
        name
    }
}

/// One DXT1 block of a single colour: both end points the same, every texel using the first.
fn solid_block(r: u8, g: u8, b: u8) -> Vec<u8> {
    let rgb565 = (u16::from(r >> 3) << 11) | (u16::from(g >> 2) << 5) | u16::from(b >> 3);
    let end = rgb565.to_le_bytes();
    vec![end[0], end[1], end[0], end[1], 0, 0, 0, 0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_bytes(width: u32, height: u32, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(data).unwrap();
        out
    }

    #[test]
    fn a_png_decodes_and_samples_with_wrapping() {
        let mut data = vec![0u8; 2 * 2 * 4];
        data[4..8].copy_from_slice(&[1, 2, 3, 4]); // x = 1, y = 0
        let picture = decode(&png_bytes(2, 2, &data)).unwrap();
        assert_eq!(picture.sample([0.75, 0.25]), [1, 2, 3, 4]);
        assert_eq!(picture.sample([1.75, -0.75]), [1, 2, 3, 4], "uv wraps in both directions");
        assert_eq!(picture.sample([0.25, 0.25]), [0, 0, 0, 0]);
    }

    #[test]
    fn a_picture_is_shrunk_to_the_limit() {
        let picture = Picture { width: 64, height: 16, rgba: vec![10; 64 * 16 * 4] };
        let level = picture.shrunk(16);
        assert_eq!((level.width, level.height), (16, 4));
    }

    #[test]
    fn a_colour_makes_one_texture() {
        let mut palette = Palette::default();
        let first = palette.add_colour([1, 2, 3]);
        assert_eq!(first, palette.add_colour([1, 2, 3]));
        assert_eq!(first, "C_010203");
        assert_eq!(palette.into_textures().len(), 1);
    }
}
