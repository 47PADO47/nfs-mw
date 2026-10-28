//! The 2012 damage model of a car in the terms of the game's hinged parts. Spec: `docs/formats/chameleon-damage.md`.
//!
//! Every car bundle has a damage clip per zone (`chameleon_vehicle::damage_clips`): 21 keys over the zone's damage, a turn
//! and a slide per bone, and a scalar that drops to 0 when the piece of the body breaks off. The game's damage model has
//! three panel levels per zone, so a part is written at the damage of those three levels (a third, two thirds, all of it):
//! the turn it has there is its `stage_angles`, and the first key at which its piece is gone is where it falls off
//! (`detach_at`). Pieces that break but are not hinged parts of the car's damage object (a lamp glass, a window, a lip of
//! a bumper) become parts of their own that do nothing but turn with the part they sit on and fall off.

use chameleon_vehicle::{BoneTrack, DamageClip, HingedPart, Skeleton};

/// The panel levels of the game's damage model as damage fractions: one third, two thirds, all.
const LEVELS: [f32; 3] = [1.0 / 3.0, 2.0 / 3.0, 1.0];
/// The stage starts written for them: just below the level, so the float division of the game reaches them.
pub const STAGES: [f32; 3] = [0.33, 0.66, 1.0];
/// A part that turns less than this at every level is not turned by the damage, degrees.
const MIN_TURN_DEG: f32 = 0.5;
/// What a piece that breaks off weighs when the damage object does not say, kg.
const PIECE_MASS_KG: f32 = 8.0;
/// The release level of a piece that is gone at the first key: not at no damage at all.
const MIN_RELEASE: f32 = 0.01;

/// How a part leaves the car, from its clip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Release {
    /// The clip does not say: the game's default (the last stage, not before half the damage).
    Default,
    /// The piece never breaks in the clip: it stays on the car.
    Never,
    /// The piece is gone from this damage fraction on.
    At(f32),
}

/// The turn of a part at the three panel levels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Swing {
    /// The axis in the 2012 engine's space (x left, y up, z forward), unit length.
    pub axis: [f32; 3],
    /// Degrees about the axis at each level, right-handed.
    pub angles: [f32; 3],
}

/// What the clips say about one part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartDamage {
    pub swing: Option<Swing>,
    pub release: Release,
}

/// The clip that moves `bone`, preferring zone `zone`, with its track.
fn find(clips: &[DamageClip], bone: usize, zone: u16) -> Option<(&DamageClip, &BoneTrack)> {
    let own = clips.iter().filter(|clip| clip.zone == zone).find_map(|clip| track_in(clip, bone));
    own.or_else(|| clips.iter().find_map(|clip| track_in(clip, bone)))
}

fn track_in(clip: &DamageClip, bone: usize) -> Option<(&DamageClip, &BoneTrack)> {
    clip.track(bone).map(|track| (clip, track))
}

/// The unit axis and signed angle (degrees) of a rotation quaternion (x, y, z, w); the axis is the x axis for no turn.
fn axis_angle(q: [f32; 4]) -> ([f32; 3], f32) {
    let sign = if q[3] < 0.0 { -1.0 } else { 1.0 };
    let (v, w) = ([q[0] * sign, q[1] * sign, q[2] * sign], q[3].abs().min(1.0));
    let sine = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if sine < 1e-6 {
        return ([1.0, 0.0, 0.0], 0.0);
    }
    ([v[0] / sine, v[1] / sine, v[2] / sine], 2.0 * sine.atan2(w).to_degrees())
}

/// The turn of `track` about `axis` at the three levels of `clip`.
fn angles_about(clip: &DamageClip, track: &BoneTrack, axis: [f32; 3]) -> [f32; 3] {
    LEVELS.map(|level| {
        let q = track.at(clip.position(level)).rotation;
        let sign = if q[3] < 0.0 { -1.0 } else { 1.0 };
        let along = (q[0] * axis[0] + q[1] * axis[1] + q[2] * axis[2]) * sign;
        2.0 * along.atan2(q[3].abs()).to_degrees()
    })
}

/// How the bone `bone` of zone `zone` turns with the damage, `None` when it turns less than [`MIN_TURN_DEG`] at every level.
pub fn swing_of(clips: &[DamageClip], bone: usize, zone: u16) -> Option<Swing> {
    let (clip, track) = find(clips, bone, zone)?;
    // The axis of the biggest turn of the three levels.
    let mut best = ([1.0, 0.0, 0.0], 0.0f32);
    for level in LEVELS {
        let (axis, angle) = axis_angle(track.at(clip.position(level)).rotation);
        if angle.abs() > best.1.abs() {
            best = (axis, angle);
        }
    }
    if best.1.abs() < MIN_TURN_DEG {
        return None;
    }
    Some(Swing { axis: best.0, angles: angles_about(clip, track, best.0) })
}

/// When the piece of `bone` leaves the car: from the first key at which it is gone. `None` for a bone no clip moves.
pub fn release_of(clips: &[DamageClip], bone: usize, zone: u16) -> Option<Release> {
    let (clip, track) = find(clips, bone, zone)?;
    let Some(key) = track.break_key() else { return Some(Release::Never) };
    Some(Release::At((key as f32 / clip.keys.saturating_sub(1).max(1) as f32).clamp(MIN_RELEASE, 1.0)))
}

/// What the clips say about the part on `bone` of zone `zone`. `real` lists the bones of the car's damage object: a piece
/// that is not one of them turns with the nearest of them above it in the skeleton, as it does on the car. `None` when no
/// clip moves the bone.
pub fn part_damage(
    clips: &[DamageClip],
    skeleton: &Skeleton,
    real: &[usize],
    bone: usize,
    zone: u16,
) -> Option<PartDamage> {
    let release = release_of(clips, bone, zone)?;
    let swing = swing_of(clips, carrier(skeleton, real, bone), zone);
    Some(PartDamage { swing, release })
}

/// The bone whose turn the part on `bone` follows: itself when it is one of `real`, else the nearest of them above it in
/// the skeleton, else itself.
pub fn carrier(skeleton: &Skeleton, real: &[usize], bone: usize) -> usize {
    if real.contains(&bone) {
        return bone;
    }
    let up = skeleton.bones.get(bone).and_then(|b| b.parent);
    up.and_then(|up| skeleton.nearest(up, real)).unwrap_or(bone)
}

/// Parts for the bones that break off but are not in `known` (the bones of the car's parts): one per bone, in bone order,
/// named `Piece<bone>`, in the zone of the clip that moves it. At most `room` of them.
pub fn pieces(clips: &[DamageClip], skeleton: &Skeleton, known: &[usize], room: usize) -> Vec<HingedPart> {
    let mut found: Vec<(usize, u16)> = Vec::new();
    for clip in clips {
        for track in clip.tracks.iter().filter(|track| track.break_key().is_some()) {
            if !known.contains(&track.bone) && found.iter().all(|(bone, _)| *bone != track.bone) {
                found.push((track.bone, clip.zone));
            }
        }
    }
    found.sort_unstable();
    found
        .into_iter()
        .filter(|(bone, _)| *bone < skeleton.bones.len())
        .take(room)
        .map(|(bone, zone)| HingedPart {
            name: format!("Piece{bone}"),
            bone,
            zone,
            centre: skeleton.bones[bone].position,
            limits: [[0.0; 2]; 3],
            mass_kg: PIECE_MASS_KG,
            thresholds: Vec::new(),
            targets: Vec::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
