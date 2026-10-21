//! The camera of a NIS scene: the scene's own track, placed at the anchor, or for a scene with no camera set a slow
//! orbit about the anchor for the scene's length. Rules: `docs/specs/nis-player.md` section 5 and `nis-player-2.md`
//! section 10.

use std::f32::consts::TAU;

use blackbox_chunk::ice::{CameraSample, Track, fov_of_lens};
use glam::{Quat, Vec3};

use crate::scenes::world::scene_space::Anchor;
use crate::viewer::camera::{Lens, original};

/// The orbit's distance from the anchor and the heights of its eye and of its look point (metres).
const ORBIT_RADIUS: f32 = 12.0;
const ORBIT_EYE_HEIGHT: f32 = 3.0;
const ORBIT_LOOK_HEIGHT: f32 = 1.5;
/// The orbit's sweep over the whole scene, in turns: a quarter turn, centred on the eye's place behind the anchor.
const ORBIT_SWEEP: f32 = 0.25;
/// The orbit's lens, in millimetres.
const ORBIT_LENS_MM: f32 = 35.0;

/// The sample of `track` at scene time `time` (seconds). The track's fraction is the time over its length.
pub fn track_sample(track: &Track, time: f32) -> Option<CameraSample> {
    let fraction = (time / track.length.max(1e-3)).clamp(0.0, 1.0);
    track.sample(fraction)
}

/// The slow orbit about the anchor at `fraction` (0 to 1) of the scene. The eye starts a little to one side of the
/// scene's back and sweeps [`ORBIT_SWEEP`] of a turn, so the camera always looks at the anchor.
pub fn orbit(anchor: &Anchor, fraction: f32, aspect: f32) -> Lens {
    let ahead = (anchor.to_render([0.0, 1.0, 0.0]) - anchor.origin).normalize_or_zero();
    let sweep = (fraction.clamp(0.0, 1.0) - 0.5) * ORBIT_SWEEP * TAU;
    let eye = anchor.origin + Quat::from_rotation_z(sweep) * (-ahead * ORBIT_RADIUS) + Vec3::Z * ORBIT_EYE_HEIGHT;
    let target = anchor.origin + Vec3::Z * ORBIT_LOOK_HEIGHT;
    let fov_bangle = f32::from(fov_of_lens(ORBIT_LENS_MM));
    original::native_lens(original::Sample { eye, target, up: Vec3::Z, fov_bangle }, aspect)
}

#[cfg(test)]
#[path = "camera_tests.rs"]
mod tests;
