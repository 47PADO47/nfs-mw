//! The active spoiler of a 2012 car. Spec: `docs/formats/chameleon-active-aero.md`.
//!
//! Some cars raise a spoiler when they are fast. The bundle holds a state machine (`HSMData`, resource type 0x501) named
//! `<car>_SpoilerAnimation`. Its conditions test the car's inputs (`speedinmph`, `gas`, `brake`, `steering`: the names the
//! executable binds to the vehicle) against numbers, and its state plays an animation (type
//! 0xB3) of the skeleton. The animation state behaviour of the executable opens when the input goes above
//! the open threshold and closes when it falls below the close threshold, advancing the animation clock forwards or
//! backwards at real time between its first key and its end.
//!
//! Only the states that are just that are read here: an animation opened by a condition on `speedinmph` (`>`) and closed by
//! one (`<`), with no timed transitions of their own. Any other state returns `None` and its part stays at rest.

use chameleon_bundle::{Bundle, ResourceKind};

use crate::skeleton::Skeleton;

/// The resource type of a state machine.
const HSM_KIND: u32 = 0x501;
/// The operator codes of a condition: 2 is "greater than" and 3 is "less than". Every speed condition of the install that
/// the condition labels name (`Speed above N`, `Speed under N`) has these codes.
const EQUAL: &str = "0";
const NOT_EQUAL: &str = "1";
const GREATER: &str = "2";
const LESS: &str = "3";
/// The input of the throttle.
const GAS_INPUT: &str = "gas";
/// The input the spoiler machines test.
const SPEED_INPUT: &str = "speedinmph";

/// Offsets in an animation resource: its length in seconds, its format and reference key, and the counts and tables.
const DURATION_AT: usize = 0x30;
const FORMAT_AT: usize = 0x34;
const REFERENCE_AT: usize = 0x36;
const KEYS_AT: usize = 0x04;
const BONES_AT: usize = 0x08;
const ROTATION_STRIDE_AT: usize = 0x1c;
const TRANSLATION_STRIDE_AT: usize = 0x20;
/// Where the offsets of the key tables and the bone maps are (a word each from here: 1 the translations, 4 their bone map,
/// 5 the packed rotations, 7 their bone map).
const TABLES_AT: usize = 0x38;
/// The id of a resource without its namespace byte: what the state machine's strings call an animation.
const ID_MASK: u64 = 0x00FF_FFFF_FFFF;
/// Smallest movement and turn taken for one: metres and degrees.
const MIN_TRAVEL: f32 = 1e-3;
const MIN_TURN_DEG: f32 = 0.05;
/// What a packed rotation key's three 10-bit fields are made of: `(field - 511) * SCALE`
/// (the executable's constants; see the spec).
const PACK_CENTRE: f32 = 511.0;
const PACK_SCALE: f32 = f32::from_bits(0x3ab5_3240);
/// The animation format that packs a rotation into 32 bits.
const PACKED_FORMAT: u16 = 1;

/// What a spoiler does: where it moves to, how long it takes and at which speeds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpoilerMotion {
    /// The bone that moves: the skeleton's `SpoilerHinge` when the animation moves it, else the first bone it moves.
    pub bone: usize,
    /// The movement of the bone between the animation's first key and its last, metres, in the engine's space (x left,
    /// y up, z forward); the bone is turned first and then moved.
    pub translation: [f32; 3],
    /// The turn between the same keys as a unit quaternion (x, y, z, w) in the engine's space; identity for none.
    pub rotation: [f32; 4],
    /// Length of the animation, seconds: the time the whole movement takes, either way.
    pub seconds: f32,
    /// The animation opens when the speed is above this, mph.
    pub above_mph: f32,
    /// It closes when the speed falls below this, mph.
    pub below_mph: f32,
    /// The throttle matters too: the animation opens only while the gas is pressed (and the speed is above `above_mph`) and
    /// closes when the gas is released; the speed alone does not close it.
    pub gas: bool,
}

impl SpoilerMotion {
    /// The turn as an axis (unit, engine space) and an angle in degrees, `None` for no turn.
    pub fn rotation_axis_angle(&self) -> Option<([f32; 3], f32)> {
        let [x, y, z, w] = self.rotation;
        let sine = (x * x + y * y + z * z).sqrt();
        let angle = 2.0 * sine.atan2(w).to_degrees();
        (angle.abs() >= MIN_TURN_DEG && sine > 1e-6).then(|| ([x / sine, y / sine, z / sine], angle))
    }
}

/// Another bone that the spoiler's animation moves (the side flaps of the Panamera, the flaps of the Agera): it moves with the
/// clock of the spoiler.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttachedMotion {
    pub bone: usize,
    /// The bone it hangs from when that bone moves too (the spoiler, or another attached one), else `None`.
    pub parent: Option<usize>,
    /// As [`SpoilerMotion::translation`] and [`SpoilerMotion::rotation`]: between the first key and the last, in the parent's
    /// frame.
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
}

/// The spoiler motion of the car in `bundle`: the animation state opened by the condition listed before it and closed by
/// the one it holds, when they are a speed above and a speed below, or the speed above with the gas pressed and the gas
/// released (see [`crate::hsm`]). `None` for a car without one, and for a state with timed transitions or other conditions,
/// which are not decoded.
pub fn spoiler_motion(bundle: &Bundle, skeleton: &Skeleton) -> Option<SpoilerMotion> {
    spoiler_state(bundle, skeleton).map(|(motion, _)| motion)
}

/// The other bones the spoiler's animation moves, parents before children.
pub fn attached_motions(bundle: &Bundle, skeleton: &Skeleton) -> Vec<AttachedMotion> {
    let Some((spoiler, block)) = spoiler_state(bundle, skeleton) else { return Vec::new() };
    let mut out: Vec<AttachedMotion> = Vec::new();
    for bone in (0..skeleton.bones.len()).filter(|&bone| bone != spoiler.bone) {
        let Some((translation, rotation, _)) = movement(&block, bone) else { continue };
        let parent = skeleton.bones[bone]
            .parent
            .filter(|&parent| parent == spoiler.bone || out.iter().any(|attached| attached.bone == parent));
        out.push(AttachedMotion { bone, parent, translation, rotation });
    }
    out
}

/// A bone played out by guarded animations (the Huayra's flaps): its movement, and the conditions that start and end the
/// animations that move it.
#[derive(Debug, Clone, PartialEq)]
pub struct FlapMotion {
    /// The bone and its movement between the first key and the last; `above_mph` and `below_mph` are 0.
    pub motion: SpoilerMotion,
    pub drives: Vec<crate::hsm::DriveSpec>,
}

/// The bones that guarded animations move, one entry for each, with every guarded animation that plays an animation that
/// moves the bone.
pub fn flap_motions(bundle: &Bundle, skeleton: &Skeleton) -> Vec<FlapMotion> {
    let kind = ResourceKind::Animation.id();
    let mut out: Vec<FlapMotion> = Vec::new();
    for index in (0..bundle.entries().len()).filter(|&at| bundle.entries()[at].kind == HSM_KIND) {
        let Ok(machine) = bundle.resource(index) else { continue };
        for drive in crate::hsm::drives(&machine.blocks[0]) {
            let Some(at) = bundle.entries().iter().position(|e| e.kind == kind && e.id & ID_MASK == drive.animation)
            else {
                continue;
            };
            let Ok(animation) = bundle.resource(at) else { continue };
            let block = &animation.blocks[0];
            for bone in (0..skeleton.bones.len()).filter(|&bone| movement(block, bone).is_some()) {
                let Some((translation, rotation, seconds)) = movement(block, bone) else { continue };
                match out.iter_mut().find(|flap| flap.motion.bone == bone) {
                    Some(flap) => flap.drives.push(drive.clone()),
                    None => out.push(FlapMotion {
                        motion: SpoilerMotion {
                            bone,
                            translation,
                            rotation,
                            seconds,
                            above_mph: 0.0,
                            below_mph: 0.0,
                            gas: false,
                        },
                        drives: vec![drive.clone()],
                    }),
                }
            }
        }
    }
    out
}

/// An air vent: a state that opens when the speed has been above a number for some seconds and closes some seconds after the
/// speed falls below another, moving one or more bones.
#[derive(Debug, Clone, PartialEq)]
pub struct VentMotion {
    /// The first bone the animation moves, with the speeds in `above_mph` (opens) and `below_mph` (closes).
    pub motion: SpoilerMotion,
    /// How long the speed has to stay above `above_mph` before the vent opens, seconds.
    pub open_seconds: f32,
    /// How long the vent waits after the speed falls below `below_mph` before it closes, seconds.
    pub close_seconds: f32,
    /// The other bones the animation moves.
    pub attached: Vec<AttachedMotion>,
}

/// The air vents of the car in `bundle` (a car has none, one or two).
pub fn vent_motions(bundle: &Bundle, skeleton: &Skeleton) -> Vec<VentMotion> {
    let kind = ResourceKind::Animation.id();
    let mut out = Vec::new();
    for index in (0..bundle.entries().len()).filter(|&at| bundle.entries()[at].kind == HSM_KIND) {
        let Ok(machine) = bundle.resource(index) else { continue };
        for vent in crate::hsm::vents(&machine.blocks[0]) {
            let Some(at) = bundle.entries().iter().position(|e| e.kind == kind && e.id & ID_MASK == vent.animation)
            else {
                continue;
            };
            let Ok(animation) = bundle.resource(at) else { continue };
            let block = &animation.blocks[0];
            let moved: Vec<usize> = (0..skeleton.bones.len()).filter(|&bone| movement(block, bone).is_some()).collect();
            let Some(&first) = moved.first() else { continue };
            let Some((translation, rotation, seconds)) = movement(block, first) else { continue };
            let motion = SpoilerMotion {
                bone: first,
                translation,
                rotation,
                seconds,
                above_mph: vent.open_mph,
                below_mph: vent.close_mph,
                gas: false,
            };
            let mut attached: Vec<AttachedMotion> = Vec::new();
            for &bone in moved.iter().skip(1) {
                let Some((translation, rotation, _)) = movement(block, bone) else { continue };
                let parent = skeleton.bones[bone]
                    .parent
                    .filter(|&parent| parent == first || attached.iter().any(|a| a.bone == parent));
                attached.push(AttachedMotion { bone, parent, translation, rotation });
            }
            out.push(VentMotion {
                motion,
                open_seconds: vent.open_seconds,
                close_seconds: vent.close_seconds,
                attached,
            });
        }
    }
    out
}

/// The speed or gas conditions of a plain state as thresholds: the speed it opens above, the speed it closes below, and
/// whether the gas matters. `None` for any other state.
fn plain_conditions(open: &[crate::hsm::Threshold], close: &[crate::hsm::Threshold]) -> Option<(f32, f32, bool)> {
    let is = |t: &crate::hsm::Threshold, input: &str, operator: &str| t.input == input && t.operator == operator;
    match (open, close) {
        ([above], [below]) if is(above, SPEED_INPUT, GREATER) && is(below, SPEED_INPUT, LESS) => {
            Some((above.value, below.value, false))
        }
        ([above, gas], [off])
            if is(above, SPEED_INPUT, GREATER) && is(gas, GAS_INPUT, NOT_EQUAL) && is(off, GAS_INPUT, EQUAL) =>
        {
            Some((above.value, 0.0, true))
        }
        _ => None,
    }
}

/// The spoiler's motion and the block of its animation.
fn spoiler_state(bundle: &Bundle, skeleton: &Skeleton) -> Option<(SpoilerMotion, Vec<u8>)> {
    let kind = ResourceKind::Animation.id();
    for index in (0..bundle.entries().len()).filter(|&at| bundle.entries()[at].kind == HSM_KIND) {
        let machine = bundle.resource(index).ok()?;
        for state in crate::hsm::states(&machine.blocks[0]) {
            let Some((above_mph, below_mph, gas)) = plain_conditions(&state.open, &state.close) else { continue };
            if state.has_children {
                continue;
            }
            let Some(at) = bundle.entries().iter().position(|e| e.kind == kind && e.id & ID_MASK == state.animation)
            else {
                continue;
            };
            let animation = bundle.resource(at).ok()?;
            let block = &animation.blocks[0];
            let hinge = skeleton.find("SpoilerHinge").filter(|&bone| movement(block, bone).is_some());
            let Some(bone) = hinge.or_else(|| (0..skeleton.bones.len()).find(|&bone| movement(block, bone).is_some()))
            else {
                continue;
            };
            let Some((translation, rotation, seconds)) = movement(block, bone) else { continue };
            let motion = SpoilerMotion { bone, translation, rotation, seconds, above_mph, below_mph, gas };
            return Some((motion, block.clone()));
        }
    }
    None
}

fn word(block: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(block.get(at..at + 4)?.try_into().ok()?))
}

fn half(block: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(block.get(at..at + 2)?.try_into().ok()?))
}

/// The movement of `bone` in an animation, between its reference key and its last: translation, rotation quaternion and the
/// animation's length. The sampler of the executable adds, to a bone's rest pose, the interpolation of two
/// keys minus the reference key; so the pose at the end of the animation is the last key less the reference. Read as the
/// executable reads it: the bone maps give the track of a bone (0xFF for none), translations are 16-byte rows by key and
/// track, rotations are 32-bit packed quaternions (format 1) by key and track.
fn movement(block: &[u8], bone: usize) -> Option<([f32; 3], [f32; 4], f32)> {
    let seconds = f32::from_bits(word(block, DURATION_AT)?);
    let (format, reference) = (half(block, FORMAT_AT)?, usize::from(half(block, REFERENCE_AT)?));
    let (keys, bones) = (word(block, KEYS_AT)? as usize, word(block, BONES_AT)? as usize);
    let (rotation_stride, translation_stride) =
        (word(block, ROTATION_STRIDE_AT)? as usize, word(block, TRANSLATION_STRIDE_AT)? as usize);
    let table = |n: usize| word(block, TABLES_AT + 4 * n).map(|at| at as usize);
    let (translations, translation_map, packed, rotation_map) = (table(1)?, table(4)?, table(5)?, table(7)?);
    if keys < 2 || bone >= bones || !(0.01..=60.0).contains(&seconds) || reference >= keys {
        return None;
    }
    let track = |map: usize| block.get(map + bone).copied().filter(|&t| t != 0xff).map(usize::from);
    let mut translation = [0.0f32; 3];
    if let Some(track) = track(translation_map) {
        let row = |key: usize| -> Option<[f32; 3]> {
            let at = translations + key * translation_stride + 16 * track;
            Some([0, 4, 8].map(|k| f32::from_bits(word(block, at + k).unwrap_or(0))))
        };
        let (last, first) = (row(keys - 1)?, row(reference)?);
        translation = [last[0] - first[0], last[1] - first[1], last[2] - first[2]];
    }
    let mut rotation = [0.0, 0.0, 0.0, 1.0];
    if let Some(track) = track(rotation_map) {
        if format != PACKED_FORMAT {
            return None;
        }
        let key = |key: usize| word(block, packed + key * rotation_stride + 4 * track).map(unpack_rotation);
        rotation = relative(key(keys - 1)?, key(reference)?);
    }
    let moved = translation.iter().any(|c| c.abs() >= MIN_TRAVEL);
    let turn = SpoilerMotion { bone, translation, rotation, seconds, above_mph: 0.0, below_mph: 0.0, gas: false };
    (moved || turn.rotation_axis_angle().is_some()).then_some((translation, rotation, seconds))
}

/// The keys of a bone in an animation: its absolute slide and turn at each key, and the animation's length in seconds.
/// `None` for a bone the animation does not move.
pub(crate) fn key_poses(block: &[u8], bone: usize) -> Option<(Vec<[f32; 3]>, Vec<[f32; 4]>, f32)> {
    let seconds = f32::from_bits(word(block, DURATION_AT)?);
    let format = half(block, FORMAT_AT)?;
    let (keys, bones) = (word(block, KEYS_AT)? as usize, word(block, BONES_AT)? as usize);
    let (rotation_stride, translation_stride) =
        (word(block, ROTATION_STRIDE_AT)? as usize, word(block, TRANSLATION_STRIDE_AT)? as usize);
    let table = |n: usize| word(block, TABLES_AT + 4 * n).map(|at| at as usize);
    let (translations, translation_map, packed, rotation_map) = (table(1)?, table(4)?, table(5)?, table(7)?);
    if keys < 2 || bone >= bones || !(0.01..=60.0).contains(&seconds) {
        return None;
    }
    let track = |map: usize| block.get(map + bone).copied().filter(|&t| t != 0xff).map(usize::from);
    let (slide, turn) = (track(translation_map), track(rotation_map));
    if slide.is_none() && turn.is_none() || turn.is_some() && format != PACKED_FORMAT {
        return None;
    }
    let mut translation = vec![[0.0f32; 3]; keys];
    let mut rotation = vec![[0.0, 0.0, 0.0, 1.0]; keys];
    for key in 0..keys {
        if let Some(track) = slide {
            let at = translations + key * translation_stride + 16 * track;
            translation[key] = [0, 4, 8].map(|k| f32::from_bits(word(block, at + k).unwrap_or(0)));
        }
        if let Some(track) = turn {
            rotation[key] = unpack_rotation(word(block, packed + key * rotation_stride + 4 * track)?);
        }
    }
    Some((translation, rotation, seconds))
}

/// A packed rotation: the top two bits say which of the four components is left out; the three 10-bit fields below hold
/// the others, in order, as `(field - 511) * 0.001382`; the left-out one is what makes the quaternion unit. The fields are
/// bits 20 to 29, 10 to 19 and 0 to 9.
pub(crate) fn unpack_rotation(packed: u32) -> [f32; 4] {
    let left_out = (packed >> 30) as usize;
    let field = |shift: u32| (((packed >> shift) & 0x3ff) as f32 - PACK_CENTRE) * PACK_SCALE;
    let stored = [field(20), field(10), field(0)];
    let w = (1.0 - stored.iter().map(|c| c * c).sum::<f32>()).max(0.0).sqrt();
    let mut out = [0.0; 4];
    let mut next = stored.into_iter();
    for (lane, slot) in out.iter_mut().enumerate() {
        *slot = if lane == left_out { w } else { next.next().unwrap_or(0.0) };
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_rotations_read_as_the_911s_do() {
        // The 911's spoiler: key 0 (the reference) is the identity, key 1 a 30.5 degree turn about the lateral axis.
        let identity = unpack_rotation(0xDFF7_FDFF);
        assert!(identity[..3].iter().all(|c| c.abs() < 2e-3) && (identity[3] - 1.0).abs() < 1e-4, "{identity:?}");
        let turned = relative(unpack_rotation(0xEBD7_FDFF), identity);
        let motion = SpoilerMotion {
            bone: 0,
            translation: [0.0; 3],
            rotation: turned,
            seconds: 0.6,
            above_mph: 70.0,
            below_mph: 40.0,
            gas: false,
        };
        let (axis, angle) = motion.rotation_axis_angle().unwrap();
        assert!((angle - 30.46).abs() < 0.05, "{angle}");
        assert!(axis[0] > 0.999, "about the lateral axis: {axis:?}");
    }

    #[test]
    fn a_quaternion_with_no_turn_has_no_axis() {
        let motion = SpoilerMotion {
            bone: 0,
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            seconds: 1.0,
            above_mph: 1.0,
            below_mph: 1.0,
            gas: false,
        };
        assert_eq!(motion.rotation_axis_angle(), None);
    }

    /// An animation block with two keys, one translation track and one rotation track, both for bone 5 of 8.
    fn animation(translations: [[f32; 3]; 2], rotations: [u32; 2]) -> Vec<u8> {
        let mut block = vec![0u8; 0x200];
        let mut put = |at: usize, value: u32| block[at..at + 4].copy_from_slice(&value.to_le_bytes());
        put(KEYS_AT, 2);
        put(BONES_AT, 8);
        put(ROTATION_STRIDE_AT, 4);
        put(TRANSLATION_STRIDE_AT, 16);
        put(DURATION_AT, 0.5f32.to_bits());
        put(FORMAT_AT, 1);
        put(TABLES_AT + 4, 0x80);
        put(TABLES_AT + 4 * 4, 0x100);
        put(TABLES_AT + 4 * 5, 0xC0);
        put(TABLES_AT + 4 * 7, 0x120);
        for (key, row) in translations.iter().enumerate() {
            for (k, value) in row.iter().enumerate() {
                put(0x80 + 16 * key + 4 * k, value.to_bits());
            }
        }
        for (key, packed) in rotations.iter().enumerate() {
            put(0xC0 + 4 * key, *packed);
        }
        block[0x100..0x108].fill(0xff);
        block[0x120..0x128].fill(0xff);
        block[0x105] = 0;
        block[0x125] = 0;
        block
    }

    #[test]
    fn a_bone_moves_by_its_last_key_less_the_reference_and_other_bones_do_not() {
        let block = animation([[0.0; 3], [0.0, 0.0203, -0.07]], [0xDFF7_FDFF, 0xEBD7_FDFF]);
        let (translation, rotation, seconds) = movement(&block, 5).unwrap();
        assert_eq!((translation, seconds), ([0.0, 0.0203, -0.07], 0.5));
        assert!(rotation[0] > 0.2 && rotation[3] > 0.9);
        assert_eq!(movement(&block, 4), None, "a bone with no track does not move");
        assert_eq!(movement(&block, 9), None, "a bone the skeleton has not");
        let still = animation([[0.0; 3]; 2], [0xDFF7_FDFF; 2]);
        assert_eq!(movement(&still, 5), None);
    }
}
