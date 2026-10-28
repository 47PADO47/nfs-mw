//! The damage animations of a 2012 car: one clip per damage zone that crumples, tilts and breaks off its body pieces.
//! Spec: `docs/formats/chameleon-damage.md`.
//!
//! The `VehicleDamageBehaviour` object imports an animation list (resource type 0xB0, 8 clips in the cars read: front,
//! rear, left, right, front left, front right, rear left, rear right, the order of the zone codes). A clip has 21 keys
//! (damage 0 to 1 in steps of 0.05) over the bones of the car's skeleton, and three kinds of track: a translation and a
//! packed rotation per bone, both additive to the rest pose as in the speed animations
//! ([`chameleon-active-aero.md`](../../../docs/formats/chameleon-active-aero.md)), and a **scalar** per bone that is 1
//! while the piece is whole and 0 once it is broken.

use chameleon_bundle::{Bundle, ResourceKind};

use crate::active_aero::unpack_rotation;
use crate::genesys::TypeNames;

/// Offset of the clip count (u16) and of the offset of the table of clip offsets (u32) in the list.
const COUNT_AT: usize = 2;
const TABLE_AT: usize = 8;
/// Fields of one clip, as the animation sampler reads them (offsets from the clip's start).
const KEYS_AT: usize = 0x04;
const BONES_AT: usize = 0x08;
const ROTATION_STRIDE_AT: usize = 0x1C;
const TRANSLATION_STRIDE_AT: usize = 0x20;
const SCALAR_STRIDE_AT: usize = 0x24;
const SECONDS_AT: usize = 0x30;
const FORMAT_AT: usize = 0x34;
const REFERENCE_AT: usize = 0x36;
const TABLES_AT: usize = 0x38;
/// Which of the clip's tables is which (word number from `TABLES_AT`).
const TRANSLATIONS: usize = 1;
const TRANSLATION_MAP: usize = 4;
const ROTATIONS: usize = 5;
const SCALAR_MAP: usize = 6;
const ROTATION_MAP: usize = 7;
const SCALARS: usize = 8;
/// A bone with no track in a map.
const NONE: u8 = 0xFF;
/// The format code that packs a rotation into 32 bits.
const PACKED_FORMAT: u16 = 1;
/// Most keys and clips a list may hold.
const MAX_KEYS: usize = 256;
const MAX_CLIPS: usize = 16;
/// A step of a piece by more than this between two keys is a piece leaving the car, metres.
pub const BREAK_STEP_M: f32 = 0.25;

/// One bone's tracks in a clip, a value per key.
#[derive(Debug, Clone, PartialEq)]
pub struct BoneTrack {
    pub bone: usize,
    /// Movement from the rest pose (a key less the clip's reference key), metres in the engine's space (x left, y up, z forward).
    pub translation: Vec<[f32; 3]>,
    /// Turn from the rest pose as a unit quaternion (x, y, z, w), about the bone's rest position.
    pub rotation: Vec<[f32; 4]>,
    /// The piece's scalar, 1 whole and 0 broken; `None` for a bone with no scalar track.
    pub scalar: Option<Vec<f32>>,
}

/// Where a bone stands at one point of a clip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BonePose {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scalar: f32,
}

/// The clip of one damage zone.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageClip {
    /// The zone code, 1 front, 2 rear, 3 left, 4 right, 5 to 8 the corners front left, front right, rear left, rear right.
    pub zone: u16,
    pub keys: usize,
    /// The clip's length in seconds (6 in every car read).
    pub seconds: f32,
    pub tracks: Vec<BoneTrack>,
}

impl BoneTrack {
    /// The pose at `position` keys from the start (0 to `keys - 1`), interpolated between the two keys around it.
    pub fn at(&self, position: f32) -> BonePose {
        let last = self.translation.len().saturating_sub(1);
        let position = position.clamp(0.0, last as f32);
        let low = (position.floor() as usize).min(last);
        let high = (low + 1).min(last);
        let t = position - low as f32;
        let lerp3 = |a: [f32; 3], b: [f32; 3]| [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * t);
        let (qa, qb) = (self.rotation[low], self.rotation[high]);
        let sign = if qa.iter().zip(&qb).map(|(a, b)| a * b).sum::<f32>() < 0.0 { -1.0 } else { 1.0 };
        let mut rotation = [0, 1, 2, 3].map(|k| qa[k] + (qb[k] * sign - qa[k]) * t);
        let length = rotation.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-9);
        rotation = rotation.map(|c| c / length);
        let scalar = self.scalar.as_ref().map_or(1.0, |s| s[low] + (s[high] - s[low]) * t);
        BonePose { translation: lerp3(self.translation[low], self.translation[high]), rotation, scalar }
    }

    /// The first key at which the piece is gone: its scalar is below one half, else it has stepped by more than
    /// [`BREAK_STEP_M`] since the key before. `None` for a piece that stays, or that is not whole to begin with.
    pub fn break_key(&self) -> Option<usize> {
        if let Some(scalar) = &self.scalar {
            return scalar.first().filter(|&&s| s >= 0.5).and_then(|_| scalar.iter().position(|&s| s < 0.5));
        }
        (1..self.translation.len()).find(|&k| distance(self.translation[k], self.translation[k - 1]) > BREAK_STEP_M)
    }
}

impl DamageClip {
    /// The track of `bone`.
    pub fn track(&self, bone: usize) -> Option<&BoneTrack> {
        self.tracks.iter().find(|track| track.bone == bone)
    }

    /// The key position (0 to `keys - 1`) of damage fraction `fraction` (0 to 1): the keys are evenly spaced.
    pub fn position(&self, fraction: f32) -> f32 {
        fraction.clamp(0.0, 1.0) * (self.keys.saturating_sub(1)) as f32
    }
}

/// `last * inverse(reference)` for unit quaternions (x, y, z, w).
fn relative(last: [f32; 4], reference: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = last;
    let [bx, by, bz, bw] = [-reference[0], -reference[1], -reference[2], reference[3]];
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn word(block: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(block.get(at..at + 4)?.try_into().ok()?) as usize)
}

fn float(block: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_bits(u32::from_le_bytes(block.get(at..at + 4)?.try_into().ok()?)))
}

/// The damage clips of a car bundle, in zone order; empty for a car with no damage animation list. The list is the
/// resource the `VehicleDamageBehaviour` object imports.
pub fn damage_clips(bundle: &Bundle) -> Vec<DamageClip> {
    let Some(list) = list_of(bundle) else { return Vec::new() };
    read_list(&list)
}

/// The main block of the animation list the damage behaviour imports.
fn list_of(bundle: &Bundle) -> Option<Vec<u8>> {
    let types = TypeNames::read(bundle).ok()?;
    let kind = ResourceKind::AnimationList.id();
    let object = ResourceKind::GenesysObject.id();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != object {
            continue;
        }
        let resource = bundle.resource(index).ok()?;
        let has_behaviour =
            resource.imports.iter().any(|import| types.name(import.id) == Some("Genesys.Gen.VehicleDamageBehaviour"));
        if !has_behaviour {
            continue;
        }
        for import in &resource.imports {
            let Some(found) = bundle.find(import.id) else { continue };
            if bundle.entries()[found].kind == kind {
                return Some(bundle.resource(found).ok()?.blocks[0].clone());
            }
        }
    }
    None
}

/// Read the clips of an animation list block.
pub fn read_list(list: &[u8]) -> Vec<DamageClip> {
    let Some(count) = list.get(COUNT_AT..COUNT_AT + 2).map(|b| usize::from(u16::from_le_bytes([b[0], b[1]]))) else {
        return Vec::new();
    };
    let Some(table) = word(list, TABLE_AT) else { return Vec::new() };
    if count == 0 || count > MAX_CLIPS {
        return Vec::new();
    }
    let mut clips = Vec::new();
    for n in 0..count {
        let Some(clip) =
            word(list, table + 4 * n).and_then(|at| list.get(at..)).and_then(|block| read_clip(block, n + 1))
        else {
            return Vec::new();
        };
        clips.push(clip);
    }
    clips
}

/// Read one clip; `block` starts at the clip.
fn read_clip(block: &[u8], zone: usize) -> Option<DamageClip> {
    let (keys, bones) = (word(block, KEYS_AT)?, word(block, BONES_AT)?);
    let seconds = float(block, SECONDS_AT)?;
    let format = u16::from_le_bytes(block.get(FORMAT_AT..FORMAT_AT + 2)?.try_into().ok()?);
    let reference = usize::from(u16::from_le_bytes(block.get(REFERENCE_AT..REFERENCE_AT + 2)?.try_into().ok()?));
    if reference >= keys
        || !(2..=MAX_KEYS).contains(&keys)
        || bones == 0
        || bones > 255
        || format != PACKED_FORMAT
        || !(0.01..=600.0).contains(&seconds)
    {
        return None;
    }
    let (rotation_stride, translation_stride, scalar_stride) =
        (word(block, ROTATION_STRIDE_AT)?, word(block, TRANSLATION_STRIDE_AT)?, word(block, SCALAR_STRIDE_AT)?);
    let table = |n: usize| word(block, TABLES_AT + 4 * n);
    let map = |n: usize, bone: usize| -> Option<usize> {
        block.get(table(n)? + bone).copied().filter(|&track| track != NONE).map(usize::from)
    };
    let mut tracks = Vec::new();
    for bone in 0..bones {
        let (slide, turn, scalar) = (map(TRANSLATION_MAP, bone), map(ROTATION_MAP, bone), map(SCALAR_MAP, bone));
        if slide.is_none() && turn.is_none() && scalar.is_none() {
            continue;
        }
        let mut track = BoneTrack {
            bone,
            translation: vec![[0.0; 3]; keys],
            rotation: vec![[0.0, 0.0, 0.0, 1.0]; keys],
            scalar: scalar.map(|_| vec![1.0; keys]),
        };
        for key in 0..keys {
            if let Some(slide) = slide {
                let at = table(TRANSLATIONS)? + key * translation_stride + 16 * slide;
                track.translation[key] = [float(block, at)?, float(block, at + 4)?, float(block, at + 8)?];
            }
            if let Some(turn) = turn {
                let at = table(ROTATIONS)? + key * rotation_stride + 4 * turn;
                track.rotation[key] = unpack_rotation(u32::try_from(word(block, at)?).ok()?);
            }
            if let (Some(scalar), Some(values)) = (scalar, track.scalar.as_mut()) {
                values[key] = float(block, table(SCALARS)? + key * scalar_stride + 4 * scalar)?;
            }
        }
        // The sampler adds a key less the reference key to the rest pose.
        let (slide0, turn0) = (track.translation[reference], track.rotation[reference]);
        for key in 0..keys {
            track.translation[key] = [0, 1, 2].map(|k| track.translation[key][k] - slide0[k]);
            track.rotation[key] = relative(track.rotation[key], turn0);
        }
        tracks.push(track);
    }
    Some(DamageClip { zone: zone as u16, keys, seconds, tracks })
}
