//! Body paint (docs/specs/car-assembly.md §7): the paint part's colour, and the flat
//! texture that stands in for the composite skin of skinnable cars.

use blackbox_carparts::Part;
use blackbox_hash::bstring_hash;
use blackbox_tpk::{AlphaUsage, PixelFormat, Texture};

use super::tables::CarTables;

/// Side of the generated paint texture (the game composites a 512×512 skin; a flat colour
/// needs no more than a few texels).
const PAINT_TEXTURE_SIZE: u32 = 4;

/// Name hash of `CARSKIN`, the light material the body groups name; drawing swaps it for the
/// paint's own material (`Paint::light_material`).
pub const CARSKIN: u32 = bstring_hash("CARSKIN");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paint {
    /// Part name, e.g. `METAL_L1_COLOR02`.
    pub name: String,
    pub rgb: [u8; 3],
    pub gloss: u8,
    /// Hash of the light material that replaces `CARSKIN` (e.g. `METPAINTSILVER`).
    pub light_material: Option<u32>,
}

impl Paint {
    pub fn of(t: &CarTables, part: &Part) -> Self {
        let channel = |name| t.parts.attribute(part, name).map_or(0, |v| v.min(255) as u8);
        Self {
            name: t.parts.name(part).to_owned(),
            rgb: [channel("RED"), channel("GREEN"), channel("BLUE")],
            gloss: channel("GLOSS"),
            light_material: t.parts.attribute(part, "LIGHT_MATERIAL_NAME"),
        }
    }

    /// A flat texture of this colour named `name` (the game fills every texel with the paint;
    /// stock cars have no vinyl on top).
    pub fn texture(&self, name: &str) -> Texture {
        let [r, g, b] = self.rgb;
        let texels = (PAINT_TEXTURE_SIZE * PAINT_TEXTURE_SIZE) as usize;
        Texture {
            name: name.to_owned(),
            name_hash: bstring_hash(name),
            width: PAINT_TEXTURE_SIZE,
            height: PAINT_TEXTURE_SIZE,
            mip_levels: 1,
            format: PixelFormat::Argb8888,
            compression_type: 0,
            alpha_usage: AlphaUsage::None,
            alpha_sorting: false,
            alpha_blend: 0,
            // A8R8G8B8 is stored B, G, R, A. The game keeps the gloss in alpha for the car
            // shader; drawn opaque here.
            data: [b, g, r, 0xFF].repeat(texels),
            palette: Vec::new(),
        }
    }

    pub fn hex(&self) -> String {
        let [r, g, b] = self.rgb;
        format!("#{r:02X}{g:02X}{b:02X}")
    }
}

/// The skin placeholders a skinnable car's body uses (`<CAR>_SKIN1`, `GLOBAL_SKIN1`).
pub fn skin_placeholders(base_model_name: &str) -> [String; 2] {
    [format!("{base_model_name}_SKIN1"), "GLOBAL_SKIN1".to_owned()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carskin_is_the_hash_cars_name() {
        // Seen in the solids of every car file as an unlisted light material (docs/specs/car-assembly.md §7).
        assert_eq!(CARSKIN, 0xD6D6_080A);
    }
}
