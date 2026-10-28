//! The `Corona` objects of the Chameleon engine: the flare, glow and beam sprites a lamp is drawn with.
//! Format and evidence: `docs/formats/chameleon-corona.md`.
//!
//! A car's lamp record (`VehicleVfxBehaviour.Corona`) names a corona object. The object holds a curve block of its own, three
//! distances, and lists of layers: beams, flares and glows. Each layer is a sprite (a material), a colour, two parameters and
//! its own curve block. The reader takes the bytes of the object's main block and the references of its import table.

pub mod curve;
pub mod look;
pub mod pattern;

#[cfg(test)]
mod tests;

pub use curve::CurveBlock;
pub use look::{RecordFactors, Seen};

use crate::{Error, f32_at, u16_at, u32_at};

/// A layer is 0xE0 bytes, and the layers of a list are contiguous.
pub const LAYER_LEN: usize = 0xE0;
/// The object's own fields: the far limit, the cull distance and a bias.
const FAR_LIMIT_AT: usize = 0xA4;
const CULL_AT: usize = 0xA8;
const BIAS_AT: usize = 0xAC;
/// The layer lists: the offset of the first layer and the offset of its u16 count.
const BEAMS: (usize, usize) = (0xB0, 0xCA);
const FLARES: (usize, usize) = (0xB4, 0xCE);
const ALT_BEAMS: (usize, usize) = (0xB8, 0xCC);
const GLOWS: (usize, usize) = (0xBC, 0xD0);
const GLOWS_2: (usize, usize) = (0xC0, 0xD2);
const GLOWS_3: (usize, usize) = (0xC4, 0xD4);
const MAX_LAYERS: usize = 16;

/// A reference in an object's import table: the offset of the slot in the main block and the id of the resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reference {
    pub at: usize,
    pub id: u64,
}

/// What a layer is drawn as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    /// A strip along the lamp's axis (the light cone seen through haze).
    Beam,
    /// A ghost sprite along the line from the lamp through the screen centre.
    Flare,
    /// A camera-facing sprite at the lamp.
    Glow,
}

/// One sprite layer of a corona.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub kind: LayerKind,
    pub curves: CurveBlock,
    /// Colour multiplier (red, green, blue, alpha) at +0xA0.
    pub colour: [f32; 4],
    /// +0xB0: a beam's start and end along the axis in metres and two more numbers; a flare's ghost offsets.
    pub params: [f32; 4],
    /// The material (a `0x2` resource of the global material dictionary) the sprite is drawn with.
    pub picture: Option<u64>,
    /// +0xC8 and +0xCC: a depth bias and a start angle.
    pub depth: f32,
    pub rotation: f32,
    /// +0xD0: 0 takes the corona record's first colour, 1 its second, anything else white.
    pub colour_source: u16,
}

/// A corona object.
#[derive(Debug, Clone, PartialEq)]
pub struct Corona {
    pub curves: CurveBlock,
    /// +0xA4: the coronas past this distance are not drawn (99,999 in most).
    pub far_limit: f32,
    /// +0xA8: a lamp farther than this from the camera is skipped (100 or 1,000).
    pub cull_distance: f32,
    pub bias: f32,
    pub beams: Vec<Layer>,
    pub flares: Vec<Layer>,
    pub glows: Vec<Layer>,
    /// The lists the other views use: beams at +0xB8, glows at +0xC0 and +0xC4.
    pub alt_beams: Vec<Layer>,
    pub glows_2: Vec<Layer>,
    pub glows_3: Vec<Layer>,
}

impl Corona {
    /// Read a corona object from the main block of its resource and its import references.
    pub fn parse(main: &[u8], refs: &[Reference]) -> Result<Self, Error> {
        let short = Error::Truncated("corona");
        let list = |(slot, count): (usize, usize), kind| layers(main, refs, slot, count, kind);
        Ok(Corona {
            curves: CurveBlock::parse(main, 0).ok_or(short.clone())?,
            far_limit: f32_at(main, FAR_LIMIT_AT).ok_or(short.clone())?,
            cull_distance: f32_at(main, CULL_AT).ok_or(short.clone())?,
            bias: f32_at(main, BIAS_AT).ok_or(short.clone())?,
            beams: list(BEAMS, LayerKind::Beam)?,
            flares: list(FLARES, LayerKind::Flare)?,
            glows: list(GLOWS, LayerKind::Glow)?,
            alt_beams: list(ALT_BEAMS, LayerKind::Beam)?,
            glows_2: list(GLOWS_2, LayerKind::Glow)?,
            glows_3: list(GLOWS_3, LayerKind::Glow)?,
        })
    }

    /// The layers the player's view draws: beams, flares and glows.
    pub fn main_view(&self) -> impl Iterator<Item = &Layer> {
        self.beams.iter().chain(&self.flares).chain(&self.glows)
    }

    /// True when the object has no layer at all.
    pub fn is_empty(&self) -> bool {
        self.main_view().next().is_none()
            && self.alt_beams.is_empty()
            && self.glows_2.is_empty()
            && self.glows_3.is_empty()
    }
}

fn layers(main: &[u8], refs: &[Reference], slot: usize, count: usize, kind: LayerKind) -> Result<Vec<Layer>, Error> {
    let short = Error::Truncated("corona layer");
    let count = usize::from(u16_at(main, count).ok_or(short.clone())?);
    if count == 0 {
        return Ok(Vec::new());
    }
    if count > MAX_LAYERS {
        return Err(Error::Truncated("corona: too many layers"));
    }
    let first = u32_at(main, slot).ok_or(short)? as usize;
    (0..count).map(|n| layer(main, refs, first + LAYER_LEN * n, kind)).collect()
}

fn layer(main: &[u8], refs: &[Reference], at: usize, kind: LayerKind) -> Result<Layer, Error> {
    let short = Error::Truncated("corona layer");
    let four = |offset: usize| -> Option<[f32; 4]> {
        Some([
            f32_at(main, at + offset)?,
            f32_at(main, at + offset + 4)?,
            f32_at(main, at + offset + 8)?,
            f32_at(main, at + offset + 12)?,
        ])
    };
    let picture = refs.iter().find(|r| r.at == at + 0xC0).map(|r| r.id);
    Ok(Layer {
        kind,
        curves: CurveBlock::parse(main, at).ok_or(short.clone())?,
        colour: four(0xA0).ok_or(short.clone())?,
        params: four(0xB0).ok_or(short.clone())?,
        picture,
        depth: f32_at(main, at + 0xC8).ok_or(short.clone())?,
        rotation: f32_at(main, at + 0xCC).ok_or(short.clone())?,
        colour_source: u16_at(main, at + 0xD0).ok_or(short)?,
    })
}
