use chameleon_bundle::Import;

use crate::bytes::{Block, import_at};
use crate::error::Result;

/// A material: the shader it runs and the textures bound to it, in the shader's sampler order.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub shader: u64,
    /// Texture resource ids, one per bound sampler.
    pub textures: Vec<u64>,
    /// The constant block the shader's parameters read from, kept raw (see the format doc for what is known).
    pub constants: Vec<u8>,
}

impl Material {
    pub fn parse(main: &[u8], imports: &[Import]) -> Result<Self> {
        let block = Block::new(main, "material");
        let shader = import_at(imports, 8, "material")?;
        let texture_count = block.u32(0x20)? as usize;
        let texture_table = block.u32(0x2C)?;
        let mut textures = Vec::with_capacity(texture_count);
        for n in 0..texture_count {
            textures.push(import_at(imports, texture_table + 4 * n as u32, "material texture")?);
        }
        let constants_at = block.u32(0x18)? as usize;
        let constants_end = block.u32(0x24)? as usize;
        let constants = match constants_end > constants_at {
            true => block.slice(constants_at, constants_end - constants_at)?.to_vec(),
            false => Vec::new(),
        };
        Ok(Self { shader, textures, constants })
    }

    /// The first float4 slot that looks like a colour: red, green and blue in 0 to 1 and alpha exactly 1. For the car
    /// shaders this is the `PbrMaterialDiffuseColour` parameter: the parameters sit in 16-byte slots in name order, but
    /// some are plain floats padded to a slot, and which slots a shader has varies, so the slot is found by its look.
    pub fn first_colour(&self) -> Option<[f32; 3]> {
        (0..self.constants.len() / 16)
            .filter_map(|slot| self.float4(slot))
            .find(|[r, g, b, a]| [r, g, b].iter().all(|c| (0.0..=1.0).contains(*c)) && *a == 1.0)
            .map(|[r, g, b, _]| [r, g, b])
    }

    /// The float4 at parameter slot `slot` of the constants, if present.
    pub fn float4(&self, slot: usize) -> Option<[f32; 4]> {
        let bytes = self.constants.get(slot * 16..slot * 16 + 16)?;
        Some(std::array::from_fn(|n| {
            f32::from_le_bytes([bytes[4 * n], bytes[4 * n + 1], bytes[4 * n + 2], bytes[4 * n + 3]])
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constants(slots: &[[f32; 4]]) -> Material {
        let bytes = slots.iter().flatten().flat_map(|value| value.to_le_bytes()).collect();
        Material { shader: 0, textures: Vec::new(), constants: bytes }
    }

    #[test]
    fn the_first_colour_skips_plain_floats_and_over_bright_slots() {
        let material =
            constants(&[[1.0, 0.0, 0.0, 0.0], [4.2, 4.2, 4.2, 1.0], [0.07, 0.07, 0.07, 1.0], [0.18, 0.18, 0.18, 1.0]]);
        assert_eq!(material.first_colour(), Some([0.07; 3]));
        assert_eq!(constants(&[[0.5, 0.0, 0.0, 0.0]]).first_colour(), None);
    }
}
