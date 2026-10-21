//! Where a scene stands in the world, shared by the cinematic moment and the NIS player: the scene's origin on the
//! ground, the turn about the up axis, and the placement of a camera track's points in render space. Rules:
//! `docs/specs/pursuit-breaker-moment.md` (section 3) and `docs/specs/nis-player.md` (section 5).

use blackbox_chunk::ice::{CameraSample, Space};
use glam::Vec3;
use nfsmw_data::world::moment::Marker;

use crate::viewer::camera::{Lens, original};

/// One quarter turn in 16-bit angle units. A scene's forward axis is its marker's angle, so the scene is turned by the
/// marker's angle less this.
const QUARTER_TURN: u16 = 16384;

/// The scene's origin and heading: where the scene is stood, and how it is turned about the up axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    pub origin: Vec3,
    /// The turn about the up axis, in 16-bit units (65536 per turn).
    pub turn: u16,
}

impl Anchor {
    /// A scene stood at `origin` (render space, on the ground) whose forward axis points along `angle` (16-bit units,
    /// counted from +x towards +y, as [`crate::scenes::world::moment::heading_angle`] gives it).
    pub fn facing(origin: Vec3, angle: u16) -> Self {
        Self { origin, turn: angle.wrapping_sub(QUARTER_TURN) }
    }

    /// A scene-space point in render space: turned about the up axis, then moved to the origin.
    pub fn to_render(self, point: [f32; 3]) -> Vec3 {
        let (sin, cos) = (f32::from(self.turn) / 65536.0 * std::f32::consts::TAU).sin_cos();
        let (x, y) = (point[0], point[1]);
        self.origin + Vec3::new(x * cos - y * sin, x * sin + y * cos, point[2])
    }
}

/// The anchor of a marker: its position stood on the ground under it (`ground` gives the ground height, or `None` to
/// keep the marker's own height), facing the marker's angle.
pub fn anchor_of(marker: Marker, ground: impl Fn(Vec3) -> Option<f32>) -> Anchor {
    let z = ground(marker.position).unwrap_or(marker.position.z);
    Anchor::facing(Vec3::new(marker.position.x, marker.position.y, z), marker.angle)
}

/// A track point in render space. Only the scene's own space is placed: car, world and hybrid spaces belong to
/// cameras that follow a car, and no moment uses them. World points are already in render space.
pub fn place(anchor: &Anchor, space: Space, point: [f32; 3]) -> Option<Vec3> {
    match space {
        Space::Scene => Some(anchor.to_render(point)),
        Space::World => Some(Vec3::from_array(point)),
        Space::Car | Space::Hybrid => None,
    }
}

/// The lens of one camera sample placed at `anchor`, `aspect` being width over height. `None` when the sample's eye or
/// look point is in a space that is not placed here.
pub fn lens_of(anchor: &Anchor, sample: &CameraSample, aspect: f32) -> Option<Lens> {
    let eye = place(anchor, sample.eye_space, sample.eye)?;
    let target = place(anchor, sample.look_space, sample.look)?;
    let forward = (target - eye).normalize_or_zero();
    let up = roll(Vec3::Z, forward, sample.roll);
    Some(original::native_lens(original::Sample { eye, target, up, fov_bangle: f32::from(sample.fov) }, aspect))
}

/// `up` turned about `forward` by `roll` (16-bit units).
pub fn roll(up: Vec3, forward: Vec3, roll: u16) -> Vec3 {
    if roll == 0 || forward == Vec3::ZERO {
        return up;
    }
    glam::Quat::from_axis_angle(forward, f32::from(roll) / 65536.0 * std::f32::consts::TAU) * up
}
