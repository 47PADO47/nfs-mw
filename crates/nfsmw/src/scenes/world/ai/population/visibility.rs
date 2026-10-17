//! What the camera sees of the traffic, and the distances the recycling rules measure. A car part counts as seen
//! when a sphere around it reaches the camera's frustum. Spec: `docs/specs/ai-traffic-spawning.md` (§8).

use blackbox_scene::{Aabb, Frustum};
use glam::Vec3;

use super::super::Focus;
use crate::scenes::world::space;
use crate::viewer::camera::Lens;

/// Extra reach (metres) on every sphere, so a part just outside the frustum edge still counts as seen.
const MARGIN: f32 = 2.0;
/// Weight `k` of the camera-eye distance in `min(|p - P1|, k |p - P0|)` (spec §8). It is 1 until the original's
/// view object is confirmed to be the camera (spec §11, open question).
const EYE_WEIGHT: f32 = 1.0;

/// What the camera sees this tick, and the distances the removal rules measure from.
#[derive(Clone, Copy)]
pub struct Viewing {
    frustum: Frustum,
    /// The camera eye, render space (`P0`).
    eye: Vec3,
    /// The player's car, render space (`P1`); the eye again when no car is driven.
    player: Vec3,
    /// The player's map position (x, y), for the rewrite's own horizontal rules.
    focus: [f32; 2],
    pub retire_distance: f32,
    pub grace: f32,
}

impl Viewing {
    pub fn new(camera: &Lens, focus: Focus) -> Self {
        let eye = camera.position;
        let player = match focus.body {
            Some(body) => space::to_render(body.position.to_array()),
            None => eye,
        };
        Self {
            frustum: Frustum::from_view_proj(&camera.view_proj()),
            eye,
            player,
            focus: focus.position,
            retire_distance: super::DESPAWN_DISTANCE,
            grace: 4.0,
        }
    }

    /// Whether a sphere of `radius` (metres) around `centre` (render space) reaches the camera's view.
    pub fn sees(&self, centre: Vec3, radius: f32) -> bool {
        sphere_seen(&self.frustum, centre, radius)
    }

    /// 3D distance (metres) from the nearer of the player's car and the camera eye.
    pub fn distance(&self, p: Vec3) -> f32 {
        p.distance(self.player).min(EYE_WEIGHT * p.distance(self.eye))
    }

    /// Horizontal distance (metres) from the player's map position.
    pub fn away(&self, p: Vec3) -> f32 {
        (p.x - self.focus[0]).hypot(p.y - self.focus[1])
    }
}

/// Whether a sphere of `radius` around `centre` may reach into the frustum. The test runs against the box around
/// the sphere, which is a superset of the sphere: it keeps a car a little longer than an exact test would, never
/// less. `MARGIN` is added on top.
pub fn sphere_seen(frustum: &Frustum, centre: Vec3, radius: f32) -> bool {
    let reach = Vec3::splat(radius + MARGIN);
    frustum.intersects(&Aabb { min: centre - reach, max: centre + reach })
}

#[cfg(test)]
mod tests {
    use blackbox_roads::Body;
    use glam::Vec2;

    use super::*;

    /// A 90 degree square lens at `eye` looking along `target` (Z up).
    fn lens(eye: Vec3, target: Vec3) -> Lens {
        Lens::look_at(eye, target, 90.0, 0.1, 1.0)
    }

    fn frustum_looking_north() -> Frustum {
        Frustum::from_view_proj(&lens(Vec3::ZERO, Vec3::Y).view_proj())
    }

    #[test]
    fn a_sphere_in_front_is_seen_and_one_behind_is_not() {
        let f = frustum_looking_north();
        assert!(sphere_seen(&f, Vec3::new(0.0, 50.0, 0.0), 1.0), "in front");
        assert!(!sphere_seen(&f, Vec3::new(0.0, -50.0, 0.0), 1.0), "far behind");
        assert!(!sphere_seen(&f, Vec3::new(0.0, -10.0, 0.0), 1.0), "behind, just out");
    }

    #[test]
    fn a_sphere_beside_the_view_is_not_seen() {
        let f = frustum_looking_north();
        // The side planes of a 90 degree lens run along the diagonals: (60, 10) is well outside.
        assert!(!sphere_seen(&f, Vec3::new(60.0, 10.0, 0.0), 1.0));
        assert!(!sphere_seen(&f, Vec3::new(0.0, 50.0, 60.0), 1.0), "above the top plane");
    }

    #[test]
    fn the_margin_admits_a_sphere_just_behind_the_near_plane() {
        let f = frustum_looking_north();
        // Centre 7.5 m behind the eye with radius 6: the sphere itself stops 1.5 m short of the eye, but the
        // margin (2 m) brings it across the near plane.
        assert!(sphere_seen(&f, Vec3::new(0.0, -7.5, 0.0), 6.0), "inside the margin");
        // Centre 9.5 m behind: beyond radius plus margin.
        assert!(!sphere_seen(&f, Vec3::new(0.0, -9.5, 0.0), 6.0), "beyond the margin");
    }

    #[test]
    fn distance_is_the_nearer_of_the_player_and_the_eye() {
        let eye = Vec3::new(0.0, 0.0, 3.0);
        let player = Vec3::new(10.0, 0.0, 0.0);
        let focus = Focus {
            position: [10.0, 0.0],
            heading: 0.0,
            speed: 0.0,
            body: Some(Body {
                position: Vec3::from(space::to_physics(player)),
                velocity: Vec3::ZERO,
                forward: Vec2::X,
                half_width: 1.0,
                half_length: 2.5,
            }),
        };
        let view = Viewing::new(&lens(eye, player), focus);
        let p = Vec3::new(20.0, 0.0, 0.0);
        let to_player = p.distance(player);
        let to_eye = p.distance(eye);
        assert!((view.distance(p) - to_player.min(to_eye)).abs() < 1e-4);
        assert!((view.away(p) - 10.0).abs() < 1e-4, "horizontal distance from the player");
    }
}
