//! The licence plate designs of the 2012 game: the backgrounds the player earns and picks in Autolog.
//! Spec: `docs/formats/chameleon-plate-designs.md`.
//!
//! A design is one record of the list object `Genesys.Gen.Online.LicensePlate` of `GAMELOGIC/GAMEPLAY.BNDL`: the number of
//! a picture (a 512 x 256 BC3 plate over a photograph, one small bundle under `UI/IMAGES/STREAMED/`), the id of the
//! design's name in the text tables, and the id of the game pack that unlocks it. The pictures are interface images: no car
//! bundle holds one, and the game's own plate mesh (see [`crate::plate`]) draws only the glyphs. What a design looks like
//! on a car is therefore a choice of whoever uses it: [`design_band`] says which part of the picture lies where the glyph
//! quads of the plate mesh are.

use chameleon_bundle::{Bundle, ResourceId, ResourceKind};
use chameleon_gfx::Texture;

use crate::error::{Error, Result};
use crate::genesys::{TypeNames, find_object};

/// The type the game registers for the list of designs.
const LIST_TYPE: &str = "Genesys.Gen.Online.LicensePlate";
/// Fields of the list: where the array of record offsets is, and how many records it has.
const LIST_ARRAY: usize = 0x10;
const LIST_COUNT: usize = 0x14;
/// Bytes of one design record, and its fields.
const RECORD: usize = 32;
const PICTURE: usize = 0x0C;
const NAME: usize = 0x14;
const PACK: usize = 0x18;
/// More designs than this is not a list of designs (the install has 666).
const MAX_DESIGNS: usize = 4096;

/// The folder of the picture bundles, each named by the decimal number of its picture.
pub const DESIGN_PICTURES: &str = "UI/IMAGES/STREAMED";
/// The key part of a picture's id: a numeric id has the class byte 1.
const NUMERIC_CLASS: u64 = 1 << 56;

/// One plate design of the install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlateDesign {
    /// The number of the picture: `UI/IMAGES/STREAMED/<picture>.BNDL`. Several designs may share one.
    pub picture: u32,
    /// The string id of the design's name in the language tables (`UI/LANGUAGE/NNNN.BNDL`).
    pub name: u32,
    /// The id of the game pack that unlocks the design: the `GamePackID` of [`crate::GamePack`].
    pub pack: u32,
}

/// Every design of the gameplay bundle `gameplay`, in the order of the game's list.
pub fn plate_designs(gameplay: &Bundle) -> Result<Vec<PlateDesign>> {
    let types = TypeNames::read(gameplay)?;
    let object = find_object(gameplay, &types, LIST_TYPE)?;
    parse_list(&object.main)
}

/// The designs listed in the main block `main` of the list object.
fn parse_list(main: &[u8]) -> Result<Vec<PlateDesign>> {
    let word = |at: usize| -> Result<usize> {
        let bytes = main.get(at..at + 4).ok_or_else(|| Error::Spec(format!("the design list ends before 0x{at:X}")))?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize)
    };
    let (array, count) = (word(LIST_ARRAY)?, word(LIST_COUNT)?);
    if count > MAX_DESIGNS {
        return Err(Error::Spec(format!("the bundle claims {count} plate designs")));
    }
    (0..count)
        .map(|n| {
            let at = word(array + 4 * n)?;
            if at + RECORD > main.len() {
                return Err(Error::Spec(format!("plate design {n} is outside the list object")));
            }
            Ok(PlateDesign {
                picture: word(at + PICTURE)? as u32,
                name: word(at + NAME)? as u32,
                pack: word(at + PACK)? as u32,
            })
        })
        .collect()
}

/// The path of the bundle that holds the picture number `picture`, relative to the install.
pub fn design_picture_path(picture: u32) -> String {
    format!("{DESIGN_PICTURES}/{picture}.BNDL")
}

/// The picture number `picture` out of its bundle `data`.
pub fn design_picture(data: &[u8], picture: u32) -> Result<Texture> {
    let bundle = Bundle::parse(data)?;
    let kind = ResourceKind::Texture.id();
    let index = bundle
        .find(NUMERIC_CLASS | u64::from(picture))
        .or_else(|| bundle.entries().iter().position(|e| e.kind == kind && ResourceId(e.id).key() == picture))
        .ok_or_else(|| Error::Spec(format!("the bundle has no picture {picture}")))?;
    let resource = bundle.resource(index)?;
    Ok(Texture::parse(&resource.blocks[0], &resource.blocks[1])?)
}

/// Width over height of the glyph strip of the plate mesh (0.264 m by 0.06 m, see `docs/formats/chameleon-plate.md`).
pub const STRIP_ASPECT: f32 = 4.4;
/// The part of the plate's width that lies inside its screws and rounded corners, as a margin on each side.
const SIDE_MARGIN: f32 = 0.08;
/// Where the middle of the strip lies on the plate, as a fraction of its height: a little below the middle, between the
/// screws, where the registration of the interface's plate is drawn.
const STRIP_CENTRE: f32 = 0.52;
/// The alpha from which a pixel belongs to the plate.
const SOLID: u8 = 128;

/// An area of a picture: `(x, y, width, height)` in pixels.
pub type Area = (usize, usize, usize, usize);

/// The area of the plate in a picture, from the alpha of its RGBA pixels (`width` x `height`, rows top to bottom). The
/// picture is a 512 x 256 texture of which the plate fills only the top left part (about 336 x 168), the rest being
/// transparent. `None` for a picture with no solid pixel.
pub fn design_plate(rgba: &[u8], width: usize, height: usize) -> Option<Area> {
    let mut bounds: Option<(usize, usize, usize, usize)> = None;
    for y in 0..height {
        for x in 0..width {
            if rgba.get((y * width + x) * 4 + 3).is_none_or(|alpha| *alpha < SOLID) {
                continue;
            }
            let (x0, y0, x1, y1) = bounds.unwrap_or((x, y, x, y));
            bounds = Some((x0.min(x), y0.min(y), x1.max(x), y1.max(y)));
        }
    }
    bounds.map(|(x0, y0, x1, y1)| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

/// The area of `plate` (see [`design_plate`]) that lies behind the glyph strip of a car's plate: the central band of the
/// plate, in the aspect [`STRIP_ASPECT`].
pub fn design_band(plate: Area) -> Area {
    let (x, y, width, height) = plate;
    let margin = (width as f32 * SIDE_MARGIN) as usize;
    let band_width = width.saturating_sub(2 * margin).max(1);
    let band_height = ((band_width as f32 / STRIP_ASPECT) as usize).clamp(1, height.max(1));
    let middle = (height as f32 * STRIP_CENTRE) as usize;
    let top = middle.saturating_sub(band_height / 2).min(height.saturating_sub(band_height));
    (x + margin.min(width.saturating_sub(band_width)), y + top, band_width, band_height)
}

#[cfg(test)]
#[path = "plate_design_tests.rs"]
mod tests;
