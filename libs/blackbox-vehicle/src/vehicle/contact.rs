//! The body box against the ground: the eight corners are tested against the road and the deepest one
//! pushes the body out and takes the impulse (spec section 4.2). This is what stops a flipped car from
//! sinking, and what holds a car whose suspension has bottomed out.

use glam::Vec3;

use super::Vehicle;
use crate::ground::Ground;
use crate::rigid_body::{BodyState, ContactParams, PlaneContact};

/// How far above a corner the ground ray starts.
const CORNER_LIFT: f32 = 0.3;

struct CornerContact {
    point: Vec3,
    normal: Vec3,
    depth: f32,
}

impl Vehicle {
    /// Resolves ground contacts of the body box and returns how many corners touch.
    pub(super) fn collide_body_with_ground(&mut self, ground: &dyn Ground) -> usize {
        if self.body.state != BodyState::Awake {
            return 0;
        }
        let rot = self.body.rotation();
        let pos = self.body.position;
        let half = self.body.dimension() + self.body.spec().collision_box_pad;
        let mut contacts: Vec<CornerContact> = Vec::with_capacity(8);
        for sx in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                for sz in [-1.0, 1.0] {
                    let point = pos + rot * (half * Vec3::new(sx, sy, sz));
                    let Some(hit) = ground.hit(point + Vec3::Y * CORNER_LIFT, Vec3::NEG_Y, CORNER_LIFT) else {
                        continue;
                    };
                    let normal = if hit.normal.y < 0.0 { -hit.normal } else { hit.normal };
                    let depth = (CORNER_LIFT - hit.distance) * normal.y;
                    if depth > 1e-4 && depth.is_finite() {
                        contacts.push(CornerContact { point, normal, depth });
                    }
                }
            }
        }
        if contacts.is_empty() {
            return 0;
        }
        contacts.sort_by(|a, b| b.depth.total_cmp(&a.depth));
        let spec = self.body.spec().clone();
        for c in &contacts {
            let n = c.normal;
            let e = Vec3::new(n.dot(rot.x_axis), n.dot(rot.y_axis), n.dot(rot.z_axis)) * spec.ground_elasticity;
            let plane = PlaneContact {
                point: c.point,
                normal: n,
                friction_static: spec.ground_friction[0],
                friction_kinetic: spec.ground_friction[1].min(spec.ground_friction[0]),
            };
            let params = ContactParams { restitution: e.length(), inertia_scale: spec.ground_moment_scale };
            if self.body.react_plane(&plane, &params).is_some() {
                break;
            }
        }
        let deepest = &contacts[0];
        self.body.position += deepest.normal * deepest.depth;
        contacts.len()
    }
}
