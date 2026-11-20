//! A recipe says how a 2012 look is made from pictures of the 2012 game; an [`Entry`] applies one to a texture of the 2005 game.
//!
//! The recipe is the look (asphalt with a white edge line, a sidewalk of slabs); the entry adds what depends on the texture it
//! replaces: its size, how bright it is lit in the 2005 game (`gain`) and whether its alpha channel is a specular mask (`alpha`).

use anyhow::{Context, Result, ensure};
use blackbox_tpk::{Level, PixelFormat, TextureSource, full_mip_count, mip_level_size};

use super::dds;
use super::layer::{Layer, Pictures, draw};
use super::resample::Rgba;

/// A picture built from layers over a plain colour.
#[derive(Debug)]
pub struct Recipe {
    /// A short name for logs and the docs table.
    pub id: &'static str,
    /// What shows where no layer covers: the colour of a base or, for a normal map, the flat normal.
    pub fill: [u8; 3],
    pub layers: &'static [Layer],
}

/// A 2005 texture and what replaces it.
#[derive(Debug)]
pub struct Entry {
    /// The name hash the game keys the texture by (`bStringHash` of the full name).
    pub hash: u32,
    /// The name the pack keeps, cut to 23 characters, for the docs and logs.
    pub name: &'static str,
    pub recipe: &'static Recipe,
    /// The size of the replacement. It keeps the shape of the 2005 texture, so the game's mapping still fits.
    pub width: u32,
    pub height: u32,
    /// 255 for a picture with no alpha (DXT1); below that, the constant alpha of a DXT5 picture, for a texture whose alpha the
    /// 2005 shaders read as a specular mask.
    pub alpha: u8,
    /// A multiplier on the colours: the 2012 pictures are lit differently from the 2005 ones, whose textures are dark.
    pub gain: f32,
}

impl Entry {
    pub const fn new(
        hash: u32,
        name: &'static str,
        recipe: &'static Recipe,
        width: u32,
        height: u32,
        alpha: u8,
        gain: f32,
    ) -> Self {
        Self { hash, name, recipe, width, height, alpha, gain }
    }

    /// The picture of the replacement, at its full size, before it is compressed.
    pub fn picture(&self, pictures: &dyn Pictures) -> Result<Rgba> {
        ensure!(self.width >= 4 && self.height >= 4, "{} is {}x{}", self.name, self.width, self.height);
        let [r, g, b] = self.recipe.fill;
        let mut canvas = Rgba::filled(self.width, self.height, [r, g, b, 255]);
        for layer in self.recipe.layers {
            draw(&mut canvas, layer, pictures).with_context(|| format!("{}: recipe {}", self.name, self.recipe.id))?;
        }
        for pixel in canvas.pixels.as_chunks_mut::<4>().0 {
            for channel in &mut pixel[..3] {
                *channel = (f32::from(*channel) * self.gain).round().clamp(0.0, 255.0) as u8;
            }
            pixel[3] = self.alpha;
        }
        Ok(canvas)
    }

    /// The `.dds` file the game reads: the picture compressed with a full chain of mips.
    pub fn dds(&self, pictures: &dyn Pictures) -> Result<Vec<u8>> {
        let picture = self.picture(pictures)?;
        let (format, four_cc) = match self.alpha {
            255 => (PixelFormat::Dxt1, *b"DXT1"),
            _ => (PixelFormat::Dxt5, *b"DXT5"),
        };
        let mips = full_mip_count(picture.width, picture.height);
        let level = Level { width: picture.width, height: picture.height, rgba: picture.pixels };
        let source = TextureSource::from_rgba(self.name, level, format, mips).context("compressing the picture")?;
        let top = mip_level_size(format, self.width, self.height, 0);
        Ok(dds::file(self.width, self.height, mips, four_cc, top, &source.data))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    struct Grey;

    impl Pictures for Grey {
        fn size(&self, _key: u32) -> Result<(u32, u32, u32)> {
            Ok((32, 32, 1))
        }

        fn level(&self, key: u32, _level: u32) -> Result<Arc<Rgba>> {
            Ok(Arc::new(Rgba::filled(32, 32, [key as u8; 4])))
        }
    }

    static BASE: Recipe = Recipe { id: "base", fill: [0, 0, 0], layers: &[Layer::new(100)] };

    #[test]
    fn the_gain_darkens_and_the_alpha_is_the_entrys() {
        let entry = Entry::new(1, "T", &BASE, 16, 16, 64, 0.5);
        let picture = entry.picture(&Grey).unwrap();
        assert_eq!(picture.at(3, 3), [50, 50, 50, 64]);
    }

    #[test]
    fn an_opaque_entry_is_dxt1_and_one_with_alpha_dxt5() {
        let opaque = Entry::new(1, "T", &BASE, 16, 16, 255, 1.0).dds(&Grey).unwrap();
        assert_eq!(&opaque[84..88], b"DXT1");
        let masked = Entry::new(1, "T", &BASE, 16, 16, 64, 1.0).dds(&Grey).unwrap();
        assert_eq!(&masked[84..88], b"DXT5");
        let texture = blackbox_tpk::read_dds("T", &masked).unwrap();
        assert_eq!((texture.width, texture.height), (16, 16));
        assert_eq!(texture.mip_levels, 5);
    }

    #[test]
    fn a_picture_too_small_for_blocks_is_refused() {
        assert!(Entry::new(1, "T", &BASE, 2, 16, 255, 1.0).picture(&Grey).is_err());
    }
}
