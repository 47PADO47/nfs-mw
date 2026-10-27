//! Textures for the 2005 car pack: the 2012 diffuse maps, cut down to a size limit, and flat colours for the parts that
//! have a colour and no map.

use std::collections::HashMap;

use blackbox_tpk::{AlphaUsage, PixelFormat, TextureSource};
use chameleon_gfx::{PixelFormat as SourceFormat, Texture, mip_size};

/// Texture bits that make a surface two-sided: the leaves of a cut-out map must show from behind.
const TWO_SIDED: u8 = 0x02;

/// The textures one car adds to its pack, by name.
#[derive(Default)]
pub struct Palette {
    textures: Vec<TextureSource>,
    names: HashMap<String, usize>,
}

impl Palette {
    pub fn into_textures(self) -> Vec<TextureSource> {
        self.textures
    }

    /// Add a copy of `texture` under `name` (once), and return the name. `None` when the format cannot be carried over.
    pub fn add_map(&mut self, name: &str, texture: &Texture, max_size: u32, cut_out: bool) -> Option<String> {
        if self.names.contains_key(name) {
            return Some(name.to_owned());
        }
        let source = convert(name, texture, max_size, cut_out)?;
        self.names.insert(name.to_owned(), self.textures.len());
        self.textures.push(source);
        Some(name.to_owned())
    }

    /// Add a flat `rgb` (linear, 0 to 1) colour texture (once), and return its name.
    pub fn add_colour(&mut self, rgb: [f32; 3]) -> String {
        let [r, g, b] = rgb.map(to_srgb_byte);
        let name = format!("C_{r:02X}{g:02X}{b:02X}");
        if !self.names.contains_key(&name) {
            self.names.insert(name.clone(), self.textures.len());
            self.textures.push(TextureSource::new(&name, 4, 4, PixelFormat::Dxt1, 1, solid_block(r, g, b)));
        }
        name
    }
}

/// The name of the translucent tint every window and lamp lens uses.
pub const GLASS: &str = "GLASS_TINT";

impl Palette {
    /// Add the glass tint (once): a dark blue-grey that lets about 60 percent of the light through, drawn blended like
    /// the stock windows. The stock `WINDOW_FRONT` is not in a pack a mod car searches, so a car carries its own.
    pub fn add_glass(&mut self) -> String {
        if !self.names.contains_key(GLASS) {
            let mut block = vec![0x66; 8]; // explicit alpha: every texel 6 of 15
            block.extend_from_slice(&solid_block(28, 38, 46)[..4]);
            block.extend_from_slice(&[0; 4]);
            let mut glass = TextureSource::new(GLASS, 4, 4, PixelFormat::Dxt3, 1, block);
            glass.alpha_usage = AlphaUsage::Modulated;
            glass.alpha_sorting = true;
            glass.alpha_blend = 1;
            self.names.insert(GLASS.to_owned(), self.textures.len());
            self.textures.push(glass);
        }
        GLASS.to_owned()
    }
}

impl Palette {
    /// Add the lit and unlit textures of the head and brake lamps of the car in `folder` (once), under the names the game
    /// swaps its lamp placeholders for (`<FOLDER>_KIT00_HEADLIGHT_ON`, `_OFF`, `_BRAKELIGHT_ON`, `_OFF`). They are flat
    /// colours: the 2012 lamps are lit strips whose pattern is not carried over.
    pub fn add_lamp_states(&mut self, folder: &str) {
        for (kind, off, on) in [("HEADLIGHT", [38, 42, 48], [255, 244, 214]), ("BRAKELIGHT", [56, 8, 8], [255, 24, 16])]
        {
            for (state, [r, g, b]) in [("OFF", off), ("ON", on)] {
                let name = format!("{folder}_KIT00_{kind}_{state}");
                if self.names.contains_key(&name) {
                    continue;
                }
                self.names.insert(name.clone(), self.textures.len());
                self.textures.push(TextureSource::new(&name, 4, 4, PixelFormat::Dxt1, 1, solid_block(r, g, b)));
            }
        }
    }
}

fn to_srgb_byte(linear: f32) -> u8 {
    let value = linear.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 { value * 12.92 } else { 1.055 * value.powf(1.0 / 2.4) - 0.055 };
    (encoded * 255.0 + 0.5) as u8
}

/// One DXT1 block of a single colour: both end points the same, every texel using the first.
fn solid_block(r: u8, g: u8, b: u8) -> Vec<u8> {
    let rgb565 = (u16::from(r >> 3) << 11) | (u16::from(g >> 2) << 5) | u16::from(b >> 3);
    let end = rgb565.to_le_bytes();
    vec![end[0], end[1], end[0], end[1], 0, 0, 0, 0]
}

/// Whether [`Palette::add_map`] can carry `texture` over: only the block formats the old engine reads.
pub fn can_carry(texture: &Texture) -> bool {
    matches!(texture.format, SourceFormat::Bc1 | SourceFormat::Bc3)
}

fn convert(name: &str, texture: &Texture, max_size: u32, cut_out: bool) -> Option<TextureSource> {
    let format = match texture.format {
        SourceFormat::Bc1 => PixelFormat::Dxt1,
        SourceFormat::Bc3 => PixelFormat::Dxt5,
        _ => return None,
    };
    let skip = levels_to_drop(texture, max_size);
    let offset: usize =
        (0..skip).map(|level| mip_size(texture.format, texture.width, texture.height, level)).sum::<Option<usize>>()?;
    let kept = texture.mip_levels - skip;
    let len: usize = (skip..texture.mip_levels)
        .map(|level| mip_size(texture.format, texture.width, texture.height, level))
        .sum::<Option<usize>>()?;
    let data = texture.data.get(offset..offset + len)?.to_vec();
    let (width, height) = ((texture.width >> skip).max(1), (texture.height >> skip).max(1));
    let mut source = TextureSource::new(name, width, height, format, kept, data);
    if cut_out && format == PixelFormat::Dxt5 {
        source.alpha_usage = AlphaUsage::PunchThrough;
        source.flags = TWO_SIDED;
    }
    Some(source)
}

/// How many of the largest mip levels to drop so the longer side is at most `max_size`.
fn levels_to_drop(texture: &Texture, max_size: u32) -> u32 {
    let longest = texture.width.max(texture.height);
    let mut skip = 0;
    while (longest >> skip) > max_size && skip + 1 < texture.mip_levels {
        skip += 1;
    }
    skip
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texture(width: u32, height: u32, mips: u32, format: SourceFormat) -> Texture {
        let size: usize = (0..mips).map(|level| mip_size(format, width, height, level).unwrap()).sum();
        Texture {
            width,
            height,
            depth: 1,
            array_size: 1,
            mip_levels: mips,
            dxgi_format: 71,
            format,
            data: vec![7; size],
        }
    }

    #[test]
    fn large_maps_lose_their_top_mips() {
        let source = convert("T", &texture(2048, 1024, 12, SourceFormat::Bc1), 1024, false).unwrap();
        assert_eq!((source.width, source.height, source.mip_count), (1024, 512, 11));
        assert_eq!(source.data.len(), source.expected_data_len());
    }

    #[test]
    fn small_maps_are_copied_whole() {
        let source = convert("T", &texture(256, 256, 9, SourceFormat::Bc3), 1024, true).unwrap();
        assert_eq!((source.width, source.mip_count), (256, 9));
        assert_eq!(source.alpha_usage, AlphaUsage::PunchThrough);
    }

    #[test]
    fn formats_the_old_engine_lacks_are_left_out() {
        assert!(convert("T", &texture(64, 64, 7, SourceFormat::Bc5), 1024, false).is_none());
    }

    #[test]
    fn the_glass_tint_is_a_blended_dxt3_texture_added_once() {
        let mut palette = Palette::default();
        assert_eq!(palette.add_glass(), palette.add_glass());
        let textures = palette.into_textures();
        assert_eq!(textures.len(), 1);
        assert_eq!(textures[0].data.len(), textures[0].expected_data_len());
        assert!(textures[0].alpha_sorting);
    }

    #[test]
    fn colours_make_one_texture_each() {
        let mut palette = Palette::default();
        let first = palette.add_colour([0.5, 0.5, 0.5]);
        assert_eq!(first, palette.add_colour([0.5, 0.5, 0.5]));
        assert_eq!(first, "C_BCBCBC");
        palette.add_colour([0.0, 0.0, 0.0]);
        assert_eq!(palette.into_textures().len(), 2);
    }

    #[test]
    fn the_lamp_states_are_four_textures_named_for_the_car() {
        let mut palette = Palette::default();
        palette.add_lamp_states("POR911");
        palette.add_lamp_states("POR911");
        let names: Vec<String> = palette.into_textures().iter().map(|t| t.name.clone()).collect();
        for name in ["HEADLIGHT_ON", "HEADLIGHT_OFF", "BRAKELIGHT_ON", "BRAKELIGHT_OFF"] {
            assert!(names.contains(&format!("POR911_KIT00_{name}")), "{names:?}");
        }
        assert_eq!(names.len(), 4);
    }
}
