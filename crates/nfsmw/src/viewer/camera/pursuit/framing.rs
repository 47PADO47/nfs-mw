//! The procedural pursuit framing: one camera pose that keeps the player and a cop in shot, from the side of the pair.
//! The original arrest uses a camera track chosen by the cutscene system (not read; see `docs/specs/camera-views.md`).

use glam::Vec3;

/// Where a pursuit shot puts the camera and what it looks at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Framing {
    pub position: Vec3,
    pub target: Vec3,
    pub fov_degrees: f32,
}

/// The field of view of a pursuit shot, degrees (a little tighter than the chase camera's rest view).
const FOV: f32 = 48.0;
/// How far the camera stands sideways from the pair: a distance that grows with the gap, kept between these, metres.
const SIDE_MIN: f32 = 6.0;
const SIDE_MAX: f32 = 16.0;
/// Metres of sideways distance per metre between the player and the cop.
const SIDE_PER_GAP: f32 = 0.8;
/// The camera's height above the pair's middle, and the height of the point it looks at, metres.
const RISE: f32 = 3.2;
const LOOK_HEIGHT: f32 = 1.0;

/// The horizontal part of a render-space vector (Z is up).
fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, v.y, 0.0)
}

/// A shot from the side of the player and `cop` (both render space, Z-up), with the pair's middle in the middle of the
/// view. The side is the one perpendicular to the line between them, so neither stands behind the other.
pub fn between(player: Vec3, cop: Vec3) -> Framing {
    let gap = flat(cop - player);
    let along = gap.normalize_or(Vec3::X);
    let side = Vec3::Z.cross(along);
    let distance = (gap.length() * SIDE_PER_GAP + SIDE_MIN).clamp(SIDE_MIN, SIDE_MAX);
    let middle = (player + cop) * 0.5;
    Framing {
        position: middle + side * distance + Vec3::Z * RISE,
        target: middle + Vec3::Z * LOOK_HEIGHT,
        fov_degrees: FOV,
    }
}
