//! The paints a car can be given: the `VehiclePaint` Genesys objects of its bundle. Spec:
//! `docs/formats/chameleon-customization.md`.
//!
//! The 2012 body-paint shader has no colour of its own: the game feeds it the colour constants of one of these records
//! (`PbrPaintDiffuseColour` and its neighbours). A car bundle carries the records of the paints the car is offered in,
//! 1 to 15 of them, and each is a 192-byte object.

use chameleon_bundle::{Bundle, ResourceKind};

/// The name the engine gives the record's type.
const TYPE_NAME: &[u8] = b"Genesys.Gen.VehiclePaint";
/// Size of one record's main block.
const RECORD_SIZE: usize = 192;

/// One paint of a car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehiclePaint {
    /// The paint's own number (`+0x74`); it is also the low bits of the resource id.
    pub id: u32,
    /// The colour of the paint, linear RGB (`+0x30`).
    pub diffuse: [f32; 3],
    /// The second diffuse colour (`+0x60`): near black for a solid paint, a lighter colour for a metallic one. The game's
    /// own swatch is the mean of this and `diffuse`.
    pub flake: [f32; 3],
    /// The secondary (pearlescent) colour (`+0x40`); black on all but one of the 443 records of the install.
    pub secondary: [f32; 3],
    /// The `Type` field (`+0x9E`): 0 for a solid paint, 1 for a metallic one **[unconfirmed meaning]**.
    pub kind: u16,
    /// The id of the swatch image in `UI/IMAGES/STREAMED/<id>.BNDL` (`+0x70`).
    pub swatch: u32,
}

impl VehiclePaint {
    /// Whether the record is the metallic kind.
    pub fn is_metallic(&self) -> bool {
        self.kind == 1
    }

    /// The paint colour as 8-bit sRGB, the way a flat colour is written into a 2005 texture.
    pub fn srgb(&self) -> [u8; 3] {
        self.diffuse.map(to_srgb_byte)
    }

    /// Read one record from the main block of its object. `None` when the block is not a record's size.
    pub fn parse(block: &[u8]) -> Option<Self> {
        if block.len() != RECORD_SIZE {
            return None;
        }
        let float = |at: usize| f32::from_le_bytes([block[at], block[at + 1], block[at + 2], block[at + 3]]);
        let word = |at: usize| u32::from_le_bytes([block[at], block[at + 1], block[at + 2], block[at + 3]]);
        let colour = |at: usize| [float(at), float(at + 4), float(at + 8)];
        let colours = [colour(0x30), colour(0x40), colour(0x60)];
        if colours.iter().flatten().any(|channel| !channel.is_finite() || !(0.0..=16.0).contains(channel)) {
            return None;
        }
        Some(Self {
            id: word(0x74),
            diffuse: colours[0],
            secondary: colours[1],
            flake: colours[2],
            kind: u16::from_le_bytes([block[0x9E], block[0x9F]]),
            swatch: word(0x70),
        })
    }
}

fn to_srgb_byte(linear: f32) -> u8 {
    let value = linear.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 { value * 12.92 } else { 1.055 * value.powf(1.0 / 2.4) - 0.055 };
    (encoded * 255.0 + 0.5) as u8
}

/// The paints of the car in `bundle`, in the bundle's resource order. A bundle without the record type has none.
pub fn paints(bundle: &Bundle) -> Vec<VehiclePaint> {
    let Some(type_id) = paint_type(bundle) else { return Vec::new() };
    let object = ResourceKind::GenesysObject.id();
    let mut out = Vec::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != object || entry.block_size(0) != RECORD_SIZE {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        if resource.imports.first().map(|import| import.id) != Some(type_id) {
            continue;
        }
        out.extend(VehiclePaint::parse(&resource.blocks[0]));
    }
    out
}

/// The id of the `VehiclePaint` type resource: the Genesys type whose block spells the type's name.
fn paint_type(bundle: &Bundle) -> Option<u64> {
    let kind = ResourceKind::GenesysType.id();
    bundle.entries().iter().enumerate().find_map(|(index, entry)| {
        if entry.kind != kind {
            return None;
        }
        let resource = bundle.resource(index).ok()?;
        let named = resource.blocks[0].windows(TYPE_NAME.len()).any(|window| window == TYPE_NAME);
        named.then_some(entry.id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(diffuse: [f32; 3], flake: [f32; 3], kind: u16) -> Vec<u8> {
        let mut block = vec![0u8; RECORD_SIZE];
        let mut put = |at: usize, values: [f32; 3]| {
            for (n, value) in values.into_iter().enumerate() {
                block[at + 4 * n..at + 4 * n + 4].copy_from_slice(&value.to_le_bytes());
            }
        };
        put(0x30, diffuse);
        put(0x60, flake);
        block[0x74..0x78].copy_from_slice(&0x0012_4626u32.to_le_bytes());
        block[0x70..0x74].copy_from_slice(&0x001b_8b63u32.to_le_bytes());
        block[0x9E..0xA0].copy_from_slice(&kind.to_le_bytes());
        block
    }

    #[test]
    fn a_record_reads_its_colours_and_kind() {
        let paint = VehiclePaint::parse(&record([0.418, 0.022, 0.025], [0.2, 0.2, 0.2], 1)).unwrap();
        assert_eq!((paint.id, paint.swatch, paint.kind), (0x12_4626, 0x1b_8b63, 1));
        assert!(paint.is_metallic());
        let [red, green, blue] = paint.srgb();
        assert!(red.abs_diff(173) <= 1 && green < 60 && blue < 60, "a red paint stays red: {red} {green} {blue}");
    }

    #[test]
    fn the_srgb_encoding_matches_the_known_endpoints() {
        assert_eq!(to_srgb_byte(0.0), 0);
        assert_eq!(to_srgb_byte(1.0), 255);
        assert_eq!(to_srgb_byte(2.0), 255, "values above 1 clip");
    }

    #[test]
    fn a_block_of_the_wrong_size_or_with_wild_numbers_is_no_record() {
        assert!(VehiclePaint::parse(&[0u8; 100]).is_none());
        assert!(VehiclePaint::parse(&record([f32::NAN, 0.0, 0.0], [0.0; 3], 0)).is_none());
        assert!(VehiclePaint::parse(&record([100.0, 0.0, 0.0], [0.0; 3], 0)).is_none());
    }
}
