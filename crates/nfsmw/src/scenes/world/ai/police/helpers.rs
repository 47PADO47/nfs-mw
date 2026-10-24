//! The helicopter's frames and the chase's per-level lookups: how the physics frame becomes the render frame, the
//! player as the helicopter sees it, and the track's height sheet as its floor. Child of police, so it reads the
//! level tables of the parent directly.

use blackbox_pursuit::helicopter::behaviour::Player;
use blackbox_pursuit::helicopter::control::{Floor, Heights};
use blackbox_roads::HeliSheet;
use glam::{Mat3, Quat, Vec3};

use super::PlayerView;
use crate::scenes::world::space;

/// A physics-frame orientation as a render-frame one. The nose is the physics X axis, up is Y; render space has x
/// forward, y left and z up (`space`), so the render frame is rebuilt from the converted nose and up.
pub(super) fn render_rotation(orientation: Quat) -> Quat {
    let nose = space::to_render((orientation * Vec3::X).to_array());
    let up = space::to_render((orientation * Vec3::Y).to_array());
    let left = up.cross(nose);
    Quat::from_mat3(&Mat3::from_cols(nose, left, up))
}

/// `v` with its height removed.
pub(super) fn horizontal(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

/// The player as the helicopter sees it: one physical pose and full body velocity.
pub(super) fn to_body(player: &PlayerView) -> Player {
    let position = Vec3::from_array(space::to_physics(player.position));
    let forward = Vec3::from_array(space::to_physics(player.forward)).normalize_or_zero();
    let velocity = Vec3::from_array(space::to_physics(player.velocity));
    Player { position, velocity, forward }
}

/// The track's height sheet as the helicopter's floor. Physics `(x, z)` is the sheet's map point `(z, -x)`
/// (`to_zone_space`); where no triangle covers the point there is no floor.
pub(super) struct SheetFloor<'a>(pub(super) Option<&'a HeliSheet>);

impl Heights for SheetFloor<'_> {
    fn elevation(&self, x: f32, z: f32) -> Option<f32> {
        self.0?.elevation(glam::Vec2::new(z, -x))
    }

    fn floor(&self, x: f32, z: f32) -> Option<Floor> {
        let surface = self.0?.surface(glam::Vec2::new(z, -x))?;
        let n = surface.normal;
        Some(Floor { height: surface.height, normal: Vec3::new(-n.y, n.z, n.x) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_roads::HeliTriangle as Triangle;

    #[test]
    fn helicopter_prediction_keeps_sideways_and_vertical_target_velocity() {
        let velocity = Vec3::new(12.0, -8.0, 4.0);
        let view = PlayerView {
            nos_input: false,
            nitrous_boost: 0.0,
            position: Vec3::ZERO,
            velocity,
            forward: Vec3::X,
            speed: 12.0,
            nitrous: 0.0,
            throttle: 0.0,
        };
        assert_eq!(to_body(&view).velocity, Vec3::from_array(space::to_physics(velocity)));
        assert_ne!(to_body(&view).velocity, to_body(&view).forward * view.speed);
    }

    #[test]
    fn sheet_floor_rotates_the_same_triangle_normal_into_physics_space() {
        let sheet = HeliSheet::new(vec![Triangle {
            plane: [glam::Vec2::ZERO, glam::Vec2::new(10.0, 0.0), glam::Vec2::new(0.0, 10.0)],
            height: [0.0, 10.0, 0.0],
        }]);
        let floor = SheetFloor(Some(&sheet)).floor(-2.0, 2.0).unwrap();
        assert!((floor.height - 2.0).abs() < 1e-5);
        assert!((floor.normal - Vec3::new(0.0, 1.0, -1.0).normalize()).length() < 1e-5);
        assert!(SheetFloor(Some(&sheet)).floor(-20.0, 20.0).is_none());
    }
}
