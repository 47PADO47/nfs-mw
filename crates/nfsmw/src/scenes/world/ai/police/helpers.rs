//! The helicopter's frames and the chase's per-level lookups: how the physics frame becomes the render frame, the
//! player as the helicopter sees it, and the track's height sheet as its floor. Child of police, so it reads the
//! level tables of the parent directly.

use blackbox_pursuit::helicopter::behaviour::Player;
use blackbox_pursuit::helicopter::control::Heights;
use blackbox_roads::HeliSheet;
use glam::{Mat3, Quat, Vec3};

use super::PlayerView;
use super::TIME_PER_LEVEL;
use crate::scenes::world::space;

/// A physics-frame orientation as a render-frame one. The nose is the physics X axis, up is Y; render space has x
/// forward, y left and z up (`space`), so the render frame is rebuilt from the converted nose and up.
pub(super) fn render_rotation(orientation: Quat) -> Quat {
    let nose = space::to_render((orientation * Vec3::X).to_array());
    let up = space::to_render((orientation * Vec3::Y).to_array());
    let left = up.cross(nose);
    Quat::from_mat3(&Mat3::from_cols(nose, left, up))
}

/// Seconds of heat growth per level: `TIME_PER_LEVEL` for the level the heat is at.
pub(super) fn time_per_level(level: u32) -> f32 {
    TIME_PER_LEVEL[(level as usize).clamp(1, TIME_PER_LEVEL.len()) - 1]
}

/// `v` with its height removed.
pub(super) fn horizontal(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

/// The player as the helicopter sees it: physics space, with its velocity along the car.
pub(super) fn to_body(player: &PlayerView) -> Player {
    let position = Vec3::from_array(space::to_physics(player.position));
    let forward = Vec3::from_array(space::to_physics(player.forward)).normalize_or_zero();
    Player { position, velocity: forward * player.speed, forward }
}

/// The track's height sheet as the helicopter's floor. Physics `(x, z)` is the sheet's map point `(z, -x)`
/// (`to_zone_space`); where no triangle covers the point there is no floor.
pub(super) struct SheetFloor<'a>(pub(super) Option<&'a HeliSheet>);

impl Heights for SheetFloor<'_> {
    fn elevation(&self, x: f32, z: f32) -> Option<f32> {
        self.0?.elevation(glam::Vec2::new(z, -x))
    }
}
