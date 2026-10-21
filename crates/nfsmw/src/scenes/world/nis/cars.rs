//! The car channels of a NIS scene: which car slot each `ZPM_Car<n>` or `ZPM_Cop<n>` channel is, the translation and
//! rotation clips that drive it, and the car's pose in render space at a scene time. Rules: `docs/specs/nis-player.md`
//! sections 5.2, 7.1, 7.2 and 7.4. The ground under the cars and the link to the real cars are in `car_world.rs`.
//!
//! The root bone. A car clip writes its pose into a DOF buffer of twelve floats per bone (`docs/specs/eagl-animation-codecs.md`):
//! the translation is at slots 8 to 10 of a bone and the rotation (xyzw) at slots 4 to 7. The car's root is the lowest
//! bone the clip writes at that offset, so [`first_at_offset`] looks for the smallest DOF index at the offset. Its
//! translation comes from the `_t` clip's buffer and its rotation from the `_q` clip's buffer.

use blackbox_anim::{Bank, Clip, DOFS_PER_BONE, Object};
use glam::{Quat, Vec3};

use crate::scenes::world::scene_space::Anchor;

/// The car slots of a scene. Slots 0 to 7 are the channels `car1` to `car8` (slot 0 is the player's car); slots 8 to 15
/// are `cop1` to `cop8`.
pub const CAR_SLOTS: usize = 16;
/// The car clips run at half the scene's time: a clip's seconds are the scene's seconds times this (`nis-player.md`
/// section 7.2, derived from the decomp's controller time scale).
pub const CAR_TIME_SCALE: f32 = 0.5;
/// The height above the ground a car is set to, in metres. The original sets ground + 5 m (`nis-player.md` section 7.4)
/// and lets its physics drop the car; the world is frozen while a scene plays here, so the car is put on the ground,
/// raised by about a wheel radius (the car's origin sits at its axles).
pub const CAR_LIFT: f32 = 0.35;
/// The first slot of the rotation (xyzw) and of the translation (xyz) within a bone's twelve DOF slots.
const ROTATION_SLOT: usize = 4;
const TRANSLATION_SLOT: usize = 8;

/// Which clip of a car channel: the translation (`_t`) or the rotation (`_q`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Translation,
    Rotation,
}

/// Which real car a car slot drives. The indices are the traffic world's car indices (`ai/nis_poses.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The player's car.
    Player,
    /// A traffic car: a civilian, or a police car the scene borrows from the chase. The chase keeps it.
    Traffic(usize),
    /// A police car the scene spawned for its cop channel. The scene removes it when it ends.
    Spawned(usize),
}

/// A car's pose in render space: where its model origin is, and how the model is turned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarSpot {
    pub position: Vec3,
    pub rotation: Quat,
}

impl CarSpot {
    /// The pose of a car whose scene-space translation and rotation are given, for a scene placed at `anchor`: the
    /// translation goes to render space through the anchor, and the rotation is turned by the anchor's turn.
    pub fn in_scene(anchor: &Anchor, translation: Vec3, rotation: Quat) -> Self {
        Self { position: anchor.to_render(translation.to_array()), rotation: anchor.rotation() * rotation }
    }
}

/// The car slot and clip part a channel name stands for. `IntroNis05ZPM_Car1_t` is slot 0, translation;
/// `EndingNis04ZPM_Cop3_q` is slot 10, rotation. The `_s` channel, a base name with no suffix and any other name give
/// `None`.
pub fn channel_of(name: &str) -> Option<(usize, Part)> {
    let (stem, part) = match (name.strip_suffix("_t"), name.strip_suffix("_q")) {
        (Some(stem), _) => (stem, Part::Translation),
        (None, Some(stem)) => (stem, Part::Rotation),
        (None, None) => return None,
    };
    let at = stem.rfind("ZPM_")? + "ZPM_".len();
    let (kind, digits) = stem.get(at..)?.split_at_checked(3)?;
    let number: usize = digits.parse().ok()?;
    let base = match kind {
        "Car" => 0,
        "Cop" => CAR_SLOTS / 2,
        _ => return None,
    };
    match (1..=CAR_SLOTS / 2).contains(&number) {
        true => Some((base + number - 1, part)),
        false => None,
    }
}

/// The smallest DOF index in `dofs` that sits at `offset` within its bone. The DOF buffer holds [`DOFS_PER_BONE`]
/// slots per bone, so the index's offset is its remainder. `None` when no index sits there.
pub fn first_at_offset(dofs: impl Iterator<Item = usize>, offset: usize) -> Option<usize> {
    dofs.filter(|dof| dof % DOFS_PER_BONE == offset).min()
}

/// The translation at DOF slot `at` (slots `at` to `at + 2`) of a DOF buffer. Zero when the buffer is too short.
pub fn read_translation(dofs: &[f32], at: usize) -> Vec3 {
    let Some(v) = dofs.get(at..at + 3) else { return Vec3::ZERO };
    Vec3::new(v[0], v[1], v[2])
}

/// The rotation at DOF slot `at` (slots `at` to `at + 3`, xyzw) of a DOF buffer, normalised. A zero or non-finite
/// quaternion, and a buffer that is too short, give the identity.
pub fn read_rotation(dofs: &[f32], at: usize) -> Quat {
    let Some(v) = dofs.get(at..at + 4) else { return Quat::IDENTITY };
    let q = Quat::from_xyzw(v[0], v[1], v[2], v[3]);
    let length_sq = q.length_squared();
    if !(length_sq > 0.0 && length_sq.is_finite()) {
        return Quat::IDENTITY;
    }
    q.normalize()
}

/// The spot with its height set to `ground` plus [`CAR_LIFT`]. Without a ground under it the spot stands as it is.
pub fn lifted(spot: CarSpot, ground: Option<f32>) -> CarSpot {
    let Some(ground) = ground else { return spot };
    CarSpot { position: Vec3::new(spot.position.x, spot.position.y, ground + CAR_LIFT), ..spot }
}

/// One car of a scene: its slot, and the translation and rotation clips of its root bone.
pub struct CarTrack {
    pub slot: usize,
    translation: Clip,
    rotation: Clip,
    /// The DOF index of the root bone's translation in the `_t` clip's buffer, and of its rotation in the `_q` clip's.
    translation_at: usize,
    rotation_at: usize,
}

impl CarTrack {
    /// The track of `slot` from its two clips. `None` when a clip writes no value at the root bone's offset.
    pub fn new(slot: usize, translation: Clip, rotation: Clip) -> Option<Self> {
        let translation_at = first_dof(&translation, TRANSLATION_SLOT)?;
        let rotation_at = first_dof(&rotation, ROTATION_SLOT)?;
        Some(Self { slot, translation, rotation, translation_at, rotation_at })
    }

    /// The car's pose at `scene_time` (seconds from the scene's zero), in render space for a scene placed at `anchor`.
    pub fn spot(&self, anchor: &Anchor, scene_time: f32) -> CarSpot {
        let secs = scene_time * CAR_TIME_SCALE;
        let translation = read_translation(&sample_dofs(&self.translation, secs), self.translation_at);
        let rotation = read_rotation(&sample_dofs(&self.rotation, secs), self.rotation_at);
        CarSpot::in_scene(anchor, translation, rotation)
    }
}

/// The car tracks of a scene's banks. A slot needs both its `_t` and its `_q` clip. A slot with only one of them, or
/// whose clips do not drive a root bone, is logged and left out, so its channel stays off.
pub fn read_tracks(banks: &[(Object, Bank)]) -> Vec<CarTrack> {
    let mut halves: Vec<(Option<Clip>, Option<Clip>)> = (0..CAR_SLOTS).map(|_| (None, None)).collect();
    for (object, bank) in banks {
        for animation in bank.iter() {
            let Some((slot, part)) = channel_of(&animation.name) else { continue };
            let clip = match Clip::read(object, animation) {
                Ok(clip) => clip,
                Err(error) => {
                    log::warn!("NIS car channel {}: {error}", animation.name);
                    continue;
                }
            };
            match part {
                Part::Translation => halves[slot].0 = Some(clip),
                Part::Rotation => halves[slot].1 = Some(clip),
            }
        }
    }
    halves.into_iter().enumerate().filter_map(|(slot, half)| track_of(slot, half)).collect()
}

/// The track of one slot from its pair of clips.
fn track_of(slot: usize, half: (Option<Clip>, Option<Clip>)) -> Option<CarTrack> {
    let (translation, rotation) = match half {
        (Some(translation), Some(rotation)) => (translation, rotation),
        (None, None) => return None,
        _ => {
            log::warn!("NIS car slot {slot}: only one of its _t and _q channels is in the bundle");
            return None;
        }
    };
    let track = CarTrack::new(slot, translation, rotation);
    if track.is_none() {
        log::warn!("NIS car slot {slot}: its channels do not drive a root bone");
    }
    track
}

/// The root bone's DOF index for `offset` in `clip`: the smallest index any channel of the clip writes there.
fn first_dof(clip: &Clip, offset: usize) -> Option<usize> {
    let dofs = clip.channels().iter().flat_map(|channel| channel.dofs().iter().map(|&dof| usize::from(dof)));
    first_at_offset(dofs, offset)
}

/// The clip sampled at `secs` into a DOF buffer of whole bones: zero where no channel writes.
fn sample_dofs(clip: &Clip, secs: f32) -> Vec<f32> {
    let bones = clip.max_dof().div_ceil(DOFS_PER_BONE).max(1);
    let mut dofs = vec![0.0; bones * DOFS_PER_BONE];
    clip.sample(secs, &mut dofs);
    dofs
}

#[cfg(test)]
#[path = "cars_tests.rs"]
mod tests;
