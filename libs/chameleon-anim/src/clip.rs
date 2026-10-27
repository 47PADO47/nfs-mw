//! One animation clip. Spec: `docs/formats/chameleon-driver-animation.md`.
//!
//! A clip is a table of keys at a fixed rate. Each key holds a packed rotation per rotation track and, for a few bones,
//! two vector tracks. A bone map says which track (if any) each bone of the skeleton has.

use crate::bytes::Block;
use crate::error::{Error, Result};
use crate::quat::{self, Quat};

const KEYS_AT: usize = 0x04;
const BONES_AT: usize = 0x08;
const ROTATION_TRACKS_AT: usize = 0x0C;
const SLIDE_TRACKS_AT: usize = 0x10;
const POSITION_TRACKS_AT: usize = 0x18;
const ROTATION_STRIDE_AT: usize = 0x1C;
const SLIDE_STRIDE_AT: usize = 0x20;
const POSITION_STRIDE_AT: usize = 0x28;
const RATE_AT: usize = 0x30;
const FORMAT_AT: usize = 0x34;
const REFERENCE_AT: usize = 0x36;
const TABLES_AT: usize = 0x38;
/// The offset of the clip's own id (low word).
const ID_AT: usize = 0x5C;
/// The table numbers: slides and their bone map, positions (share the slides' map), packed rotations and their bone map.
const SLIDES: usize = 1;
const POSITIONS: usize = 2;
const SLIDE_MAP: usize = 4;
const ROTATIONS: usize = 5;
const ROTATION_MAP: usize = 7;
/// The packed-rotation format.
const PACKED: u16 = 1;
/// A bone map entry for "no track".
const NO_TRACK: u8 = 0xFF;

/// A clip, decoded.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    /// The low word of the clip's id in the character bundle.
    pub id: u32,
    pub keys: usize,
    /// Keys per second.
    pub rate: f32,
    /// The key the clip's other keys are relative to when it is used as an addition.
    pub reference: usize,
    /// Bones of the skeleton the clip was made for.
    pub bones: usize,
    /// The rotation track of each bone, if it has one.
    pub rotation_track: Vec<Option<u8>>,
    /// The slide / position track of each bone, if it has one.
    pub slide_track: Vec<Option<u8>>,
    /// `rotations[key][track]`
    pub rotations: Vec<Vec<Quat>>,
    /// `slides[key][track]`: small offsets (metres) of the bones that have a vector track.
    pub slides: Vec<Vec<[f32; 3]>>,
    /// `positions[key][track]`: where the bones with a vector track are (metres, the figure's space).
    pub positions: Vec<Vec<[f32; 3]>>,
}

impl Clip {
    /// Parse one clip from its bytes (from its start to the start of the next clip or the end of the list).
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let block = Block::new(bytes, "animation clip");
        let (keys, bones) = (block.u32(KEYS_AT)? as usize, block.u32(BONES_AT)? as usize);
        if keys == 0 || keys > 4096 || bones == 0 || bones > 255 {
            return Err(Error::Invalid { what: "animation clip", detail: format!("{keys} keys over {bones} bones") });
        }
        let format = block.u16(FORMAT_AT)?;
        if format != PACKED {
            return Err(Error::Invalid { what: "animation clip", detail: format!("rotation format {format}") });
        }
        let table = |n: usize| block.u32(TABLES_AT + 4 * n).map(|at| at as usize);
        let map = |at: usize| -> Result<Vec<Option<u8>>> {
            let raw = block.slice(at, bones)?;
            Ok(raw.iter().map(|&track| (track != NO_TRACK).then_some(track)).collect())
        };
        let rotation_tracks = block.u32(ROTATION_TRACKS_AT)? as usize;
        let slide_tracks = block.u32(SLIDE_TRACKS_AT)? as usize;
        let position_tracks = block.u32(POSITION_TRACKS_AT)? as usize;
        let rotation_stride = block.u32(ROTATION_STRIDE_AT)? as usize;
        let slide_stride = block.u32(SLIDE_STRIDE_AT)? as usize;
        let position_stride = block.u32(POSITION_STRIDE_AT)? as usize;
        let (rotations_at, slides_at, positions_at) = (table(ROTATIONS)?, table(SLIDES)?, table(POSITIONS)?);
        let mut rotations = Vec::with_capacity(keys);
        let mut slides = Vec::with_capacity(keys);
        let mut positions = Vec::with_capacity(keys);
        for key in 0..keys {
            let row = rotations_at + key * rotation_stride;
            rotations
                .push((0..rotation_tracks).map(|t| block.u32(row + 4 * t).map(quat::unpack)).collect::<Result<_>>()?);
            let row = slides_at + key * slide_stride;
            slides.push((0..slide_tracks).map(|t| block.f32s::<3>(row + 16 * t)).collect::<Result<_>>()?);
            let row = positions_at + key * position_stride;
            positions.push((0..position_tracks).map(|t| block.f32s::<3>(row + 16 * t)).collect::<Result<_>>()?);
        }
        Ok(Self {
            id: block.u32(ID_AT)?,
            keys,
            rate: block.f32(RATE_AT)?,
            reference: usize::from(block.u16(REFERENCE_AT)?),
            bones,
            rotation_track: map(table(ROTATION_MAP)?)?,
            slide_track: map(table(SLIDE_MAP)?)?,
            rotations,
            slides,
            positions,
        })
    }

    /// The length in seconds, first key to last.
    pub fn seconds(&self) -> f32 {
        (self.keys - 1) as f32 / self.rate.max(1e-3)
    }

    /// The two keys around `time` (seconds, clamped to the clip) and the fraction between them.
    pub fn locate(&self, time: f32) -> (usize, usize, f32) {
        let at = (time * self.rate).clamp(0.0, (self.keys - 1) as f32);
        let low = (at.floor() as usize).min(self.keys - 1);
        let high = (low + 1).min(self.keys - 1);
        (low, high, at - low as f32)
    }

    /// The rotation of `bone` at `time` seconds, `None` when the clip does not turn it.
    pub fn rotation(&self, bone: usize, time: f32) -> Option<Quat> {
        let track = usize::from((*self.rotation_track.get(bone)?)?);
        let (low, high, t) = self.locate(time);
        Some(quat::slerp(self.rotations[low][track], self.rotations[high][track], t))
    }

    /// The position of `bone` at `time` seconds, `None` when the clip has none for it.
    pub fn position(&self, bone: usize, time: f32) -> Option<[f32; 3]> {
        let track = usize::from((*self.slide_track.get(bone)?)?);
        let (low, high, t) = self.locate(time);
        let (a, b) = (self.positions.get(low)?.get(track)?, self.positions.get(high)?.get(track)?);
        Some(std::array::from_fn(|n| a[n] + (b[n] - a[n]) * t))
    }
}
